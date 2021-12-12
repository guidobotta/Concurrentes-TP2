use super::env::Envs;
use common::dns::DNS;
use common::protocolo_transaccion::{CodigoTransaccion, MensajeTransaccion, ProtocoloTransaccion};
use rand::Rng;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// EstadoServicio representa el estado de la transacción de cierto id.
/// # Variantes
/// Ready: simboliza el estado ready luego de obtener los recursos exitosamente.
/// Commit: simboliza el estado commit luego de recibir un mensaje de commit.
/// Abort: simboliza el estado abort luego de recibir un mensaje de abort o de
/// haber fallado al obtener los recursos.
enum EstadoServicio {
    Ready,
    Commit,
    Abort,
}

/// WebService implementa el flujo principal del WebService. Realiza la
/// comunicación con el nodo lider de alglobo y simula trabajo y el éxito o
/// fracaso al intentar obtener los recursos en un prepare.
pub struct WebService {
    id: usize,
    protocolo: ProtocoloTransaccion,
    log: HashMap<usize, EstadoServicio>,
    envs: Envs,
}

impl WebService {
    /// Devuelve una instancia de WebService.
    /// Recibe un id. Cada id representa un servicio:
    /// - 0 para la aerolinea
    /// - 1 para el hotel
    /// - 2 para el banco
    pub fn new(id: usize) -> Self {
        WebService {
            log: HashMap::new(),
            protocolo: ProtocoloTransaccion::new(DNS::direccion_webservice(&id)).unwrap(),
            id,
            envs: Envs::get_envs("./files/env.json"),
        }
    }

    /// Corre el flujo principal del programa cíclicamente.
    pub fn run(&mut self) {
        loop {
            let mensaje = self.protocolo.recibir(None).unwrap(); // TODO: revisar el timeout

            match mensaje.codigo {
                CodigoTransaccion::PREPARE { monto } => self.responder_prepare(mensaje, monto),
                CodigoTransaccion::COMMIT => self.responder_commit(mensaje),
                CodigoTransaccion::ABORT => self.responder_abort(mensaje),
                _ => println!(
                    "[WEBSERVICE] Recibí algo que no puedo interpretar de {}",
                    mensaje.id_emisor
                ),
            }
        }
    }

    // TODO: Documentacion?? Es privada
    fn responder_prepare(&mut self, mensaje: MensajeTransaccion, monto: f64) {
        println!(
            "[WEBSERVICE] Recibí PREPARE de {} para el pago {} con monto {}",
            mensaje.id_emisor, mensaje.id_op, monto
        );
        let respuesta_ready =
            MensajeTransaccion::new(CodigoTransaccion::READY, self.id, mensaje.id_op);
        let respuesta_commit =
            MensajeTransaccion::new(CodigoTransaccion::COMMIT, self.id, mensaje.id_op);
        let respuesta_abort =
            MensajeTransaccion::new(CodigoTransaccion::ABORT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) {
            match estado {
                EstadoServicio::Ready => self.insertar_y_enviar(
                    EstadoServicio::Ready,
                    respuesta_ready,
                    mensaje.id_emisor,
                ),
                EstadoServicio::Commit => self.insertar_y_enviar(
                    EstadoServicio::Commit,
                    respuesta_commit,
                    mensaje.id_emisor,
                ),
                EstadoServicio::Abort => self.insertar_y_enviar(
                    EstadoServicio::Abort,
                    respuesta_abort,
                    mensaje.id_emisor,
                ),
            }

            return;
        };

        self.simular_trabajo();

        match self.simular_resultado() {
            Ok(_) => {
                self.insertar_y_enviar(EstadoServicio::Ready, respuesta_ready, mensaje.id_emisor)
            }
            Err(_) => {
                self.insertar_y_enviar(EstadoServicio::Abort, respuesta_abort, mensaje.id_emisor)
            }
        };
    }

    // TODO: Documentacion?? Es privada
    fn responder_commit(&mut self, mensaje: MensajeTransaccion) {
        println!(
            "[WEBSERVICE] Recibí COMMIT de {} para el pago {}",
            mensaje.id_emisor, mensaje.id_op
        );

        let respuesta = MensajeTransaccion::new(CodigoTransaccion::COMMIT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) {
            match estado {
                EstadoServicio::Ready => {
                    self.simular_trabajo();
                    self.insertar_y_enviar(EstadoServicio::Commit, respuesta, mensaje.id_emisor);
                }
                EstadoServicio::Commit => {
                    self.insertar_y_enviar(EstadoServicio::Commit, respuesta, mensaje.id_emisor)
                }
                EstadoServicio::Abort => {
                    println!("[WEBSERVICE] Error inesperado: llego commit con estado abort")
                }
            }
        };
    }

    // TODO: Documentacion?? Es privada
    fn responder_abort(&mut self, mensaje: MensajeTransaccion) {
        println!(
            "[WEBSERVICE] Recibí ABORT de {} para el pago {}",
            mensaje.id_emisor, mensaje.id_op
        );

        let respuesta = MensajeTransaccion::new(CodigoTransaccion::ABORT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) {
            match estado {
                EstadoServicio::Ready => {
                    self.simular_trabajo();
                    self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor);
                }
                EstadoServicio::Commit => {
                    println!("[WEBSERVICE] Error inesperado: llego abort con estado commit")
                }
                EstadoServicio::Abort => {
                    self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor)
                }
            }

            return;
        };

        // Llega abort sin estado, no puede pasar porque se maneja en alglobo
        self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor);
    }

    // TODO: Documentacion?? Es privada
    fn insertar_y_enviar(
        &mut self,
        estado: EstadoServicio,
        mensaje: MensajeTransaccion,
        id_emisor: usize,
    ) {
        self.log.insert(mensaje.id_op, estado);
        let direccion = DNS::direccion_alglobo(&id_emisor);

        println!("[WEBSERVICE] Envío {:?} a {}", mensaje.codigo, id_emisor);
        let enviado = self.protocolo.enviar(&mensaje, direccion);
        if enviado.is_err() {
            println!("[WEBSERVICE] Error: Fallo al enviar mensaje")
        }
    }

    // TODO: Documentacion?? Es privada
    fn simular_trabajo(&self) {
        let mut rng = rand::thread_rng();
        let tiempo_trabajo = rng.gen_range(self.envs.trabajo_min..self.envs.trabajo_max);
        thread::sleep(Duration::from_millis(tiempo_trabajo));
    }

    // TODO: Documentacion?? Es privada
    fn simular_resultado(&self) -> Result<(), ()> {
        let mut rng = rand::thread_rng();
        let ok = rng.gen::<f32>() >= self.envs.probabilidad_fallo;

        if ok {
            Ok(())
        } else {
            Err(())
        }
    }
}
