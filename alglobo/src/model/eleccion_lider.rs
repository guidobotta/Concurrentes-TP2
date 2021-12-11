use common::mensaje_lider::{CodigoLider, MensajeLider};
use common::protocolo::Protocolo;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use common::dns::DNS;

const TEAM_MEMBERS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(8); // <- Si pasa este tiempo me hago lider

pub struct EleccionLider {
    id: usize,
    protocolo: Protocolo,
    id_lider: Arc<(Mutex<Option<usize>>, Condvar)>,
    obtuve_ok: Arc<(Mutex<bool>, Condvar)>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    respondedor: Option<JoinHandle<()>>
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
            respondedor: None
        };

        let mut clone = ret.clone();
        ret.respondedor = Some(thread::spawn(move || clone.responder()));

        ret.buscar_nuevo_lider();
        ret
    }

    pub fn bloquear_si_no_soy_lider(&self) -> bool {
        self.id_lider
            .1
            .wait_while(self.id_lider.0.lock().unwrap(), |id_lider| {
                id_lider.is_none() || id_lider.unwrap() != self.id
            })
            .unwrap()
            .unwrap();

        true
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

    pub fn notificar_finalizacion(&mut self) {
        let mensaje = MensajeLider::new(CodigoLider::ELECCION, self.id);
        for peer_id in 0..TEAM_MEMBERS {
            if peer_id != self.id {
                let _ = self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
            } 
        }
    }

    pub fn buscar_nuevo_lider(&mut self) {
        if *self.stop.0.lock().unwrap() {
            return;
        }

        if self.id_lider.0.lock().unwrap().is_none() {
            // ya esta buscando lider
            return;
        }

        println!("[ELECCION]: En busca de un lider");

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

    fn enviar_eleccion(&mut self) {
        // P envía el mensaje ELECTION a todos los procesos que tengan número mayor
        thread::sleep(Duration::from_millis(500)); // TODO: CAMBIAR ESTO
        let mensaje = MensajeLider::new(CodigoLider::ELECCION, self.id);
        for peer_id in (self.id + 1)..TEAM_MEMBERS {
            self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
        }
    }

    fn anunciarme_lider(&mut self) {
        // El nuevo coordinador se anuncia con un mensaje COORDINATOR
        println!("[ELECCION]: Me anuncio como lider");
        let mensaje = MensajeLider::new(CodigoLider::COORDINADOR, self.id);

        for peer_id in 0..TEAM_MEMBERS {
            if peer_id != self.id {
                self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
            }
        }
        
        *self.id_lider.0.lock().unwrap() = Some(self.id);
        self.id_lider.1.notify_all();
    }

    fn responder(&mut self) {
        while !*self.stop.0.lock().unwrap() {
            // TODO: revisar el timeout
            if let Ok(mensaje) = self.protocolo.recibir_lider(Some(Duration::from_millis(10000))) { // <- Tolerancia a recibir un mensaje
                let id_emisor = mensaje.id_emisor;
                
                // TODO: ver si pasamos las conexiones a TCP
                match mensaje.codigo {
                    CodigoLider::OK => {
                        //println!("[ELECCION {}] recibí OK de {}", self.id, id_emisor);
                        *self.obtuve_ok.0.lock().unwrap() = true;
                        self.obtuve_ok.1.notify_all();
                    }
                    CodigoLider::ELECCION => {
                        //println!("[ELECCION {}] recibí ELECCION de {}", self.id, id_emisor);
                        if id_emisor < self.id {
                            // TODO: Sacar a una función auxiliar
                            let mensaje = MensajeLider::new(CodigoLider::OK, self.id);
                            self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&id_emisor));
                            
                            let mut me = self.clone();
                            thread::spawn(move || me.buscar_nuevo_lider()); // TODO: revisar esto
                        }
                    }
                    CodigoLider::COORDINADOR => {
                        println!("[ELECCION]: El nuevo lider es {}", id_emisor);
                        *self.id_lider.0.lock().unwrap() = Some(id_emisor);
                        self.id_lider.1.notify_all();
                        
                        let mut me = self.clone();
                        thread::spawn(move || me.mantener_vivo()); // TODO: revisar esto
                    }
                    CodigoLider::VERIFICAR => {
                        //println!("[ELECCION {}] recibí VERIFICAR de {}", self.id, id_emisor);
                        if self.soy_lider() {
                            let mensaje = MensajeLider::new(CodigoLider::OK, self.id);
                            self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&id_emisor));
                        }
                    }
                    _ => {
                        println!("[ELECCION]: Recibí algo que no puedo interpretar de {}", id_emisor);
                    }
                };
            } else {
                // Hubo timeout, por lo tanto no recibí nada
                let mut me = self.clone();
                if !self.soy_lider() { // TODO: Posible Deadlock
                    thread::spawn(move || me.buscar_nuevo_lider()); // TODO: revisar esto
                }
            }

            // if *self.stop.0.lock().unwrap() {
            //     break;
            // }
        }

        *self.stop.0.lock().unwrap() = false;
        self.stop.1.notify_all();
    }

    fn mantener_vivo(&mut self) {
        while !self.soy_lider() { // CAMBIAR LOOP INFINITO, VER COMO USAR EL STOP            
            let mensaje = MensajeLider::new(CodigoLider::VERIFICAR, self.id);
            
            //println!("[ELECCION {}] envío VERIFICAR", self.id); // TODO: TENEMOS EL IDLIDER, PODEMOS ENVIARLE SOLO A EL

            for peer_id in (self.id + 1)..TEAM_MEMBERS {
                self.protocolo.enviar_lider(&mensaje, DNS::direccion_lider(&peer_id));
            }

            thread::sleep(Duration::from_millis(2000)); // TODO: revisar esto
        }
    }

    pub fn finalizar(&mut self) { // TODO: ver si usar y donde
        self.notificar_finalizacion();
        *self.stop.0.lock().unwrap() = true;
        // TODO: Ver si este codigo comentado es necesario.
        //self.stop
        //    .1
        //    .wait_while(self.stop.0.lock().unwrap(), |should_stop| *should_stop);
        if let Some(res) = self.respondedor.take() { let _ = res.join(); }
    }

    pub fn clone(&self) -> EleccionLider {
        EleccionLider {
            id: self.id,
            protocolo: self.protocolo.clone(),
            id_lider: self.id_lider.clone(),
            obtuve_ok: self.obtuve_ok.clone(),
            stop: self.stop.clone(),
            respondedor: None
        }
    }
}
