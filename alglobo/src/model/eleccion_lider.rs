use common::mensaje_lider::{CodigoLider, MensajeLider};
use common::protocolo::Protocolo;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use common::dns::DNS;

const TEAM_MEMBERS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct EleccionLider {
    id: usize,
    protocolo: Protocolo,
    id_lider: Arc<(Mutex<Option<usize>>, Condvar)>,
    obtuve_ok: Arc<(Mutex<bool>, Condvar)>,
    stop: Arc<(Mutex<bool>, Condvar)>,
}

impl EleccionLider {
    pub fn new(id: usize) -> EleccionLider {
        let protocolo = Protocolo::new(DNS::direccion_lider(&id)).unwrap();

        let mut ret = EleccionLider {
            id,
            protocolo,
            id_lider: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            obtuve_ok: Arc::new((Mutex::new(false), Condvar::new())),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
        };

        let mut clone = ret.clone();
        thread::spawn(move || clone.responder());

        ret.buscar_nuevo_lider();
        ret
    }

    pub fn soy_lider(&self) -> bool {
        self.get_id_lider() == self.id
    }

    pub fn get_id_lider(&self) -> usize {
        self.id_lider
            .1
            .wait_while(self.id_lider.0.lock().unwrap(), |id_lider| {
                id_lider.is_none()
            })
            .unwrap()
            .unwrap()
    }

    pub fn buscar_nuevo_lider(&mut self) {
        if *self.stop.0.lock().unwrap() {
            return;
        }

        if self.id_lider.0.lock().unwrap().is_none() {
            // ya esta buscando lider
            return;
        }

        println!("[{}] buscando lider", self.id);

        *self.obtuve_ok.0.lock().unwrap() = false;
        *self.id_lider.0.lock().unwrap() = None;

        self.enviar_eleccion();
        let obtuve_ok =
            self.obtuve_ok
                .1
                .wait_timeout_while(self.obtuve_ok.0.lock().unwrap(), TIMEOUT, |got_it| !*got_it);
                
        if !*obtuve_ok.unwrap().0 {
            self.anunciarme_lider()
        } else {
            self.id_lider
                .1
                .wait_while(self.id_lider.0.lock().unwrap(), |id_lider| {
                    id_lider.is_none()
                });
        }
    }

    fn id_a_mensaje(&self, header: u8) -> Vec<u8> {
        let mut msg = vec![header];
        msg.extend_from_slice(&self.id.to_le_bytes());
        msg
    }

    fn enviar_eleccion(&mut self) {
        // P envía el mensaje ELECTION a todos los procesos que tengan número mayor
        let mensaje = MensajeLider::new(CodigoLider::ELECCION, self.id);
        for peer_id in (self.id + 1)..TEAM_MEMBERS {
            self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
        }
    }

    fn anunciarme_lider(&mut self) {
        // El nuevo coordinador se anuncia con un mensaje COORDINATOR
        println!("[{}] me anuncio como lider", self.id);
        let mensaje = MensajeLider::new(CodigoLider::COORDINADOR, self.id);

        for peer_id in 0..TEAM_MEMBERS {
            if peer_id != self.id {
                self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
            }
        }
        
        *self.id_lider.0.lock().unwrap() = Some(self.id);
    }

    fn responder(&mut self) {
        while !*self.stop.0.lock().unwrap() {
            let mensaje = self.protocolo.recibir_lider(None).unwrap(); // TODO: revisar el timeout
            let id_emisor = mensaje.id_emisor;

            if *self.stop.0.lock().unwrap() {
                break;
            }

            match mensaje.codigo {        
                CodigoLider::OK => {
                    println!("[ELECCION {}] recibí OK de {}", self.id, id_emisor);
                    *self.obtuve_ok.0.lock().unwrap() = true;
                    self.obtuve_ok.1.notify_all();
                }
                CodigoLider::ELECCION => {
                    println!("[ELECCION {}] recibí ELECCION de {}", self.id, id_emisor);
                    if id_emisor < self.id {
                        // TODO: Sacar a una función auxiliar
                        let mensaje = MensajeLider::new(CodigoLider::OK, self.id);
                        self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&id_emisor));
                        
                        let mut me = self.clone();
                        thread::spawn(move || me.buscar_nuevo_lider());
                    }
                }
                CodigoLider::COORDINADOR => {
                    println!("[ELECCION {}] recibí COORDINADOR de {}", self.id, id_emisor);
                    *self.id_lider.0.lock().unwrap() = Some(id_emisor);
                    self.id_lider.1.notify_all();
                }
                _ => {
                    println!("[ELECCION {}] recibí algo que no puedo interpretar {}", self.id, id_emisor);
                }
            }
        }

        *self.stop.0.lock().unwrap() = false;
        self.stop.1.notify_all();
    }

    fn stop(&mut self) {
        *self.stop.0.lock().unwrap() = true;
        self.stop
            .1
            .wait_while(self.stop.0.lock().unwrap(), |should_stop| *should_stop);
    }

    fn clone(&self) -> EleccionLider {
        EleccionLider {
            id: self.id,
            protocolo: self.protocolo.clone(),
            id_lider: self.id_lider.clone(),
            obtuve_ok: self.obtuve_ok.clone(),
            stop: self.stop.clone(),
        }
    }
}
