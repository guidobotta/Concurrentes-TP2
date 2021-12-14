use super::log::{EstadoTransaccion, Log, Transaccion};
use common::dns::DNS;
use common::error::{ErrorApp, ErrorInterno, Resultado};
use common::protocolo_transaccion::{CodigoTransaccion, MensajeTransaccion, ProtocoloTransaccion};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

/// Cantidad de webservices disponibles
const WEBSERVICES: usize = 3;
/// Tolerancia a recibir las respuestas de todos los webservices
const TIMEOUT_WEBSERVICES: Duration = Duration::from_secs(4);

/// CoordinadorTransaccion implementa el manejo de transacciones a través del
/// envío y recepción de mensajes con los distintos webservices.
pub struct CoordinadorTransaccion {
    log: Arc<RwLock<Log>>,
    protocolo: ProtocoloTransaccion,
    respuestas: Arc<(Mutex<Vec<Option<MensajeTransaccion>>>, Condvar)>,
    id: usize,
    destinatarios: Vec<String>,
    continuar: Arc<AtomicBool>,
    respondedor: Option<JoinHandle<()>>,
}

impl CoordinadorTransaccion {
    /// Devuelve una instancia de CoordinadorTransaccion.
    /// Recibe el id asociado al nodo de alglobo y un Log.
    pub fn new(id: usize, log: Arc<RwLock<Log>>) -> Resultado<Self> {
        let protocolo = ProtocoloTransaccion::new(DNS::direccion_alglobo(&id))?;
        let respuestas = Arc::new((Mutex::new(vec![None; WEBSERVICES]), Condvar::new()));
        let continuar = Arc::new(AtomicBool::new(true));
        let ret = CoordinadorTransaccion {
            log,
            protocolo: protocolo.clone()?,
            respuestas: respuestas.clone(),
            id,
            destinatarios: vec![0, 1, 2]
                .iter()
                .map(|id| DNS::direccion_webservice(id))
                .collect(),
            continuar: continuar.clone(),
            respondedor: Some(thread::spawn(move || {
                CoordinadorTransaccion::responder(protocolo, respuestas, continuar)
            })),
        };

        Ok(ret)
    }

    /// Finaliza al coordinador
    pub fn finalizar(&mut self) {
        self.continuar.store(false, Ordering::Relaxed);
        if let Some(res) = self.respondedor.take() {
            let _ = res.join();
        }
    }

    /// Recibe una transaccion y la procesa
    pub fn submit(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        let trans_en_log = self
            .log
            .read()
            .expect("Error al tomar lock del log en Coordinador")
            .obtener(&transaccion.id);
        match trans_en_log {
            None => self.full_protocol(transaccion),
            Some(t) => match t.estado {
                EstadoTransaccion::Prepare => self.full_protocol(transaccion),
                EstadoTransaccion::Commit => self.commit(transaccion),
                EstadoTransaccion::Abort => {
                    let _ = self.abort(transaccion);
                    Err(ErrorApp::Interno(ErrorInterno::new("Transaccion abortada")))
                }
                EstadoTransaccion::Finalize => Ok(()),
            },
        }
    }

    /// Ejecuta el protocolo completo para la transaccion
    fn full_protocol(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        match self.prepare(transaccion) {
            Ok(_) => self.commit(transaccion),
            Err(e) => {
                let _ = self.abort(transaccion);
                Err(e)
            }
        }
    }

    /// Ejecuta el prepare para la transaccion
    fn prepare(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.prepare());
        println!("[Coordinador]: Prepare de transaccion {}", transaccion.id);

        let id_op = transaccion.id;
        let pago = transaccion
            .get_pago()
            .expect("Intento de ejecutar transaccion sin pago");
        // Preparo los mensajes a enviar
        let m_hotel = MensajeTransaccion::new(
            CodigoTransaccion::PREPARE {
                monto: pago.get_monto_hotel(),
            },
            self.id,
            id_op,
        );
        let m_aerolinea = MensajeTransaccion::new(
            CodigoTransaccion::PREPARE {
                monto: pago.get_monto_aerolinea(),
            },
            self.id,
            id_op,
        );
        let m_banco = MensajeTransaccion::new(
            CodigoTransaccion::PREPARE {
                monto: pago.get_monto_hotel() + pago.get_monto_aerolinea(),
            },
            self.id,
            id_op,
        );

        // Mensaje esperado
        let esperado = MensajeTransaccion::new(CodigoTransaccion::READY, self.id, id_op);

        self.send_and_wait(vec![m_hotel, m_aerolinea, m_banco], esperado, false)
    }

    /// Ejecuta el commit para la transaccion
    fn commit(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.commit());
        println!("[Coordinador]: Commit de transaccion {}", transaccion.id);
        let id_op = transaccion.id;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = MensajeTransaccion::new(CodigoTransaccion::COMMIT, self.id, id_op);

        let res = self.send_and_wait(
            vec![mensaje.clone(), mensaje.clone(), mensaje.clone()],
            mensaje,
            true,
        );

        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.finalize());
        println!("[Coordinador]: Finalize de transaccion {}", transaccion.id);
        res
    }

    /// Ejecuta el abort para la transaccion
    fn abort(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.abort());
        println!("[Coordinador]: Abort de transaccion {}", transaccion.id);

        let id_op = transaccion.id;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = MensajeTransaccion::new(CodigoTransaccion::ABORT, self.id, id_op);

        let res = self.send_and_wait(
            vec![mensaje.clone(), mensaje.clone(), mensaje.clone()],
            mensaje,
            true,
        );

        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.finalize());
        println!("[Coordinador]: Finalize de transaccion {}", transaccion.id);
        res
    }

    /// Envia el mensaje a todos los destinatarios y espera por sus respuestas
    /// con un timeout definido. En caso de timeout vuelve a enviar.
    fn send_and_wait(
        &mut self,
        mensajes: Vec<MensajeTransaccion>,
        esperado: MensajeTransaccion,
        mensaje_critico: bool,
    ) -> Resultado<()> {
        loop {
            let respuestas;
            *self
                .respuestas
                .0
                .lock()
                .expect("Error al tomar lock de respuestas en Coordinador") =
                vec![None; WEBSERVICES];

            for (idx, mensaje) in mensajes.iter().enumerate() {
                self.protocolo
                    .enviar(mensaje, self.destinatarios[idx].clone())?;
            }
            respuestas = self.respuestas.1.wait_timeout_while(
                self.respuestas
                    .0
                    .lock()
                    .expect("Error al tomar lock de respuestas en Coordinador"),
                TIMEOUT_WEBSERVICES,
                |respuestas| respuestas.iter().any(Option::is_none),
            );

            let mensajes_esperados = match &respuestas {
                Ok(val) if !val.1.timed_out() => respuestas
                    .expect("Error al tomar lock de respuestas en Coordinador")
                    .0
                    .iter()
                    .all(|opt| opt.as_ref().map_or(false, |r| r == &esperado)),
                _ => {
                    println!(
                        "[Coordinador] Timeout de recepcion a webservices, reintentando id {}",
                        esperado.id_op
                    );
                    continue;
                }
            };

            if mensajes_esperados {
                break;
            } else if mensaje_critico {
                continue;
            } else {
                return Err(ErrorApp::Interno(ErrorInterno::new(
                    "Respuesta no esperada",
                )));
            }
        }

        Ok(())
    }

    /// Recibe mensajes de los webservices y guarda el resultado.
    fn responder(
        mut protocolo: ProtocoloTransaccion,
        respuestas: Arc<(Mutex<Vec<Option<MensajeTransaccion>>>, Condvar)>,
        continuar: Arc<AtomicBool>,
    ) {
        while continuar.load(Ordering::Relaxed) {
            let mensaje = match protocolo.recibir(Some(TIMEOUT_WEBSERVICES)) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let id_emisor = mensaje.id_emisor;
            match mensaje.codigo {
                CodigoTransaccion::READY | CodigoTransaccion::COMMIT | CodigoTransaccion::ABORT => {
                    println!(
                        "[Coordinador] Recibí {:?} de {} para la transaccion {}",
                        mensaje.codigo, id_emisor, mensaje.id_op
                    );
                    respuestas
                        .0
                        .lock()
                        .expect("Error al tomar lock de respuestas en Coordinador")[id_emisor] =
                        Some(mensaje);
                    respuestas.1.notify_all();
                }
                _ => {
                    println!(
                        "[Coordinador]: Recibí algo que no puedo interpretar de {}",
                        id_emisor
                    );
                }
            }
        }
    }
}

/// Finaliza al coordinador
impl Drop for CoordinadorTransaccion {
    fn drop(&mut self) {
        self.finalizar();
    }
}
