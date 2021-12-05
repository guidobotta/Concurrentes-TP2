use std::time::Duration;
use std::collections::HashMap;
use std::thread;
use common::protocolo::Protocolo;
use common::mensaje::{Mensaje, CodigoMensaje};
use common::dns::DNS;
use rand::Rng;

enum EstadoServicio {
    Ready,
    Commit,
    Abort
}
pub struct WebService {
    id: usize,
    protocolo: Protocolo,
    log: HashMap<usize, EstadoServicio>
}

impl WebService {
    pub fn new(id: usize) -> Self {
        WebService {
            log: HashMap::new(),
            protocolo: Protocolo::new(DNS::direccion_webservice(&id)).unwrap(),
            id
        }
    }

    pub fn run(&mut self) {
        loop {
            let mensaje = self.protocolo.recibir(None).unwrap(); // TODO: revisar el timeout

            match mensaje.codigo {
                CodigoMensaje::PREPARE {monto} => self.responder_prepare(mensaje, monto),
                CodigoMensaje::COMMIT => self.responder_commit(mensaje),
                CodigoMensaje::ABORT => self.responder_abort(mensaje),
                _ => println!("[COORDINATOR] recibí algo que no puedo interpretar {}", mensaje.id_emisor)
            }
        }
    }

    fn responder_prepare(&mut self, mensaje: Mensaje, monto: f64) {
        println!("[COORDINATOR] recibí PREPARE de {} con monto {}", mensaje.id_emisor, monto);
        
        let respuesta_ready = Mensaje::new(CodigoMensaje::READY, self.id, mensaje.id_op);
        let respuesta_commit = Mensaje::new(CodigoMensaje::COMMIT, self.id, mensaje.id_op);
        let respuesta_abort = Mensaje::new(CodigoMensaje::ABORT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) { // TODO: cuidado con el id_op cuando se quiera reintentar va a fallar
            match estado {
                EstadoServicio::Ready => self.insertar_y_enviar(EstadoServicio::Ready, respuesta_ready, mensaje.id_emisor),
                EstadoServicio::Commit => self.insertar_y_enviar(EstadoServicio::Commit, respuesta_commit, mensaje.id_emisor),
                EstadoServicio::Abort => self.insertar_y_enviar(EstadoServicio::Abort, respuesta_abort, mensaje.id_emisor)
            }

            return;
        };

        self.simular_trabajo();
                
        match self.simular_resultado() {
          Ok(_) => self.insertar_y_enviar(EstadoServicio::Ready, respuesta_ready, mensaje.id_emisor),
          Err(_) =>  self.insertar_y_enviar(EstadoServicio::Abort, respuesta_abort, mensaje.id_emisor)
        };
    }
    
    fn responder_commit(&mut self, mensaje: Mensaje) {
        println!("[COORDINATOR] recibí COMMIT de {}", mensaje.id_emisor);

        let respuesta = Mensaje::new(CodigoMensaje::COMMIT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) { // TODO: cuidado con el id_op cuando se quiera reintentar va a fallar
            match estado {
                EstadoServicio::Ready => {
                    self.simular_trabajo();
                    self.insertar_y_enviar(EstadoServicio::Commit, respuesta, mensaje.id_emisor);
                },
                EstadoServicio::Commit => self.insertar_y_enviar(EstadoServicio::Commit, respuesta, mensaje.id_emisor),
                EstadoServicio::Abort => println!("Error inesperado: llego commit con estado abort")
            }
        };
    }
    
    fn responder_abort(&mut self, mensaje: Mensaje) {
        println!("[COORDINATOR] recibí ABORT de {}", mensaje.id_emisor);

        let respuesta = Mensaje::new(CodigoMensaje::ABORT, self.id, mensaje.id_op);

        if let Some(estado) = self.log.get(&mensaje.id_op) { // TODO: cuidado con el id_op cuando se quiera reintentar va a fallar
            match estado {
                EstadoServicio::Ready => {
                    self.simular_trabajo();
                    self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor);
                },
                EstadoServicio::Commit => println!("Error inesperado: llego abort con estado commit"),
                EstadoServicio::Abort => self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor)
            }

            return;
        };

        // Llega abort sin estado, no puede pasar porque se maneja en alglobo
        self.insertar_y_enviar(EstadoServicio::Abort, respuesta, mensaje.id_emisor);
    }

    fn insertar_y_enviar(&mut self, estado: EstadoServicio, mensaje: Mensaje, id_emisor: usize) {
        self.log.insert(mensaje.id_op, estado);

        let direccion = DNS::direccion_alglobo(&id_emisor);
        let _ = self.protocolo.enviar(&mensaje, direccion); // TODO: manejar errores
    }

    fn simular_trabajo(&self) {
        let mut rng = rand::thread_rng();
        let tiempo_trabajo = rng.gen_range(300..1000); // TODO: env
        thread::sleep(Duration::from_millis(tiempo_trabajo));
    }
    
    fn simular_resultado(&self) -> Result<(), ()> {
        let mut rng = rand::thread_rng();
        let ok = rng.gen::<f32>() >= 0.1; // TODO: env
    
        if ok { Ok(()) } else { Err(()) }
    }
}

