use super::log::{EstadoTransaccion, Log, Transaccion};
use common::dns::DNS;
use common::error::{ErrorApp, ErrorInterno, Resultado};
use common::protocolo_transaccion::{CodigoTransaccion, MensajeTransaccion, ProtocoloTransaccion};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

const WEBSERVICES: usize = 3;
const TIMEOUT_WEBSERVICES: Duration = Duration::from_secs(4);

/// CoordinadorTransaccion implementa el manejo de transacciones a través del
/// envío y recepción de mensajes con los distintos webservices.
pub struct CoordinadorTransaccion {
    log: Arc<RwLock<Log>>,
    protocolo: ProtocoloTransaccion,
    responses: Arc<(Mutex<Vec<Option<MensajeTransaccion>>>, Condvar)>,
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
        let responses = Arc::new((Mutex::new(vec![None; WEBSERVICES]), Condvar::new()));
        let continuar = Arc::new(AtomicBool::new(true));
        let ret = CoordinadorTransaccion {
            log,
            protocolo: protocolo.clone()?,
            responses: responses.clone(),
            id,
            destinatarios: vec![0, 1, 2]
                .iter()
                .map(|id| DNS::direccion_webservice(&id))
                .collect(), // TODO: cambiar esto
            continuar: continuar.clone(),
            respondedor: Some(thread::spawn(move || {
                CoordinadorTransaccion::responder(protocolo, responses, continuar)
            })),
        };

        Ok(ret)
    }

    // TODO: Documentacion
    pub fn finalizar(&mut self) {
        self.continuar.store(false, Ordering::Relaxed); //Ver si el Ordering Relaxed esta bien
        if let Some(res) = self.respondedor.take() {
            let _ = res.join();
        }
    }

    // TODO: Documentacion
    pub fn submit(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        let trans_en_log = self
            .log
            .read()
            .expect("Error al tomar lock del log en Coordinador")
            .obtener(&transaccion.id);
        match trans_en_log {
            None => self.full_protocol(transaccion),
            Some(t) => match t.estado {
                EstadoTransaccion::Prepare => self.full_protocol(transaccion), //TODO: Ver esto
                EstadoTransaccion::Commit => self.commit(transaccion),
                EstadoTransaccion::Abort => {
                    let _ = self.abort(transaccion);
                    return Err(ErrorApp::Interno(ErrorInterno::new("Transaccion abortada")));
                }
            },
        }
    }

    // TODO: Documentacion?? Es privada
    fn full_protocol(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        match self.prepare(transaccion) {
            Ok(_) => self.commit(transaccion), // TODO: ver que hacer con el result de estos (quizas reintentar commit)
            Err(e) => {
                let _ = self.abort(transaccion); // TODO: ver que hacer con el result de estos y tema de escribir para reintentar
                Err(e)
            }
        }
    }

    // TODO: Documentacion?? Es privada
    fn prepare(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.prepare());
        println!("[COORDINADOR]: Prepare de transaccion {}", transaccion.id);

        let id_op = transaccion.id;
        let pago = transaccion
            .get_pago()
            .expect("Intento de ejecutar transaccion sin pago");
        //TODO: Hay que cambiar en el Mensaje el id_op por id_transaccion, sino no vamos a poder reintentar.
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
        ); // TODO: PASAR ESTO AL PARSER

        // Mensaje esperado
        let esperado = MensajeTransaccion::new(CodigoTransaccion::READY, self.id, id_op); // TODO: PASAR ESTO AL PARSER

        self.send_and_wait(vec![m_hotel, m_aerolinea, m_banco], esperado)
    }

    // TODO: Documentacion?? Es privada
    fn commit(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.commit());
        println!("[COORDINADOR]: Commit de transaccion {}", transaccion.id);
        let id_op = transaccion.id;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = MensajeTransaccion::new(CodigoTransaccion::COMMIT, self.id, id_op); // TODO: pensar si hacer que esperado sea finished o no

        self.send_and_wait(
            vec![mensaje.clone(), mensaje.clone(), mensaje.clone()],
            mensaje,
        )
    }

    // TODO: Documentacion?? Es privada
    fn abort(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log
            .write()
            .expect("Error al tomar lock del log en Coordinador")
            .insertar(transaccion.abort());
        println!("[COORDINADOR]: Abort de transaccion {}", transaccion.id);

        let id_op = transaccion.id;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = MensajeTransaccion::new(CodigoTransaccion::ABORT, self.id, id_op);

        self.send_and_wait(
            vec![mensaje.clone(), mensaje.clone(), mensaje.clone()],
            mensaje,
        )
    }

    // TODO: Documentacion?? Es privada
    //Queremos que se encargue de enviarle los mensajes a los destinatarios
    //y espere por sus respuestas, con un timeout y numero de intentos dado
    fn send_and_wait(
        &mut self,
        mensajes: Vec<MensajeTransaccion>,
        esperado: MensajeTransaccion,
    ) -> Resultado<()> {
        loop {
            let responses;
            *self
                .responses
                .0
                .lock()
                .expect("Error al tomar lock de responses en Coordinador") =
                vec![None; WEBSERVICES];

            for (idx, mensaje) in mensajes.iter().enumerate() {
                self.protocolo
                    .enviar(&mensaje, self.destinatarios[idx].clone())?;
            }
            responses = self.responses.1.wait_timeout_while(
                self.responses
                    .0
                    .lock()
                    .expect("Error al tomar lock de responses en Coordinador"),
                TIMEOUT_WEBSERVICES,
                |responses| responses.iter().any(Option::is_none),
            );

            match &responses {
                Ok(val) if !val.1.timed_out() => {}
                _ => {
                    println!(
                        "[COORDINADOR] Timeout de recepcion a webservices, reintentando id {}",
                        esperado.id_op
                    );
                    continue;
                }
            };

            if !responses
                .expect("Error al tomar lock de responses en Coordinador")
                .0
                .iter()
                .all(|opt| opt.as_ref().map_or(false, |r| r == &esperado))
            {
                return Err(ErrorApp::Interno(ErrorInterno::new(
                    "Respuesta no esperada",
                )));
            }
            break;
        }

        Ok(())
    }

    // TODO: Documentacion?? Es privada
    fn responder(
        mut protocolo: ProtocoloTransaccion,
        responses: Arc<(Mutex<Vec<Option<MensajeTransaccion>>>, Condvar)>,
        continuar: Arc<AtomicBool>,
    ) {
        while continuar.load(Ordering::Relaxed) {
            let mensaje = match protocolo.recibir(Some(TIMEOUT_WEBSERVICES)) {
                Ok(m) => m,
                Err(_) => continue,
            }; // TODO: Revisar si hacer esto es correcto
            let id_emisor = mensaje.id_emisor;
            match mensaje.codigo {
                CodigoTransaccion::READY | CodigoTransaccion::COMMIT | CodigoTransaccion::ABORT => {
                    //println!("[COORDINATOR] recibí READY de {}", id_emisor);
                    responses
                        .0
                        .lock()
                        .expect("Error al tomar lock de responses en Coordinador")[id_emisor] =
                        Some(mensaje);
                    responses.1.notify_all();
                }
                _ => {
                    println!(
                        "[COORDINADOR]: Recibí algo que no puedo interpretar de {}",
                        id_emisor
                    );
                }
            }
        }
    }
}

// TODO: Documentacion
impl Drop for CoordinadorTransaccion {
    fn drop(&mut self) {
        self.finalizar();
    }
}
