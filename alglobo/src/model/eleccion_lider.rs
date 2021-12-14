use common::dns::DNS;
use common::error::Resultado;
use common::protocolo_lider::{CodigoLider, MensajeLider, ProtocoloLider};
use std::sync::atomic::{Ordering, AtomicBool};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TEAM_MEMBERS: usize = 5;
const TIMEOUT_LIDER: Duration = Duration::from_secs(6); // <- Si pasa este tiempo me hago lider
const TIMEOUT_MENSAJE: Duration = Duration::from_secs(10); // <- Tolerancia a recibir un mensaje
const TIMEOUT_MANTENER_VIVO: Duration = Duration::from_secs(2); // <- Frecuencia de enviado del keep alive

/// EleccionLider implementa la eleccion del lider y se encarga de mantener
/// siempre un único lider activo a través del envío y recepción de mensajes
/// con las distintas réplicas.
pub struct EleccionLider {
    id: usize,
    protocolo: ProtocoloLider,
    id_lider: Arc<(Mutex<Option<usize>>, Condvar)>,
    obtuve_ok: Arc<(Mutex<bool>, Condvar)>,
    stop: Arc<AtomicBool>,
    respondedor: Option<JoinHandle<()>>,
}

impl EleccionLider {
    /// Devuelve una instancia de EleccionLider.
    /// Recibe el id asociado al nodo de alglobo.
    pub fn new(id: usize) -> Resultado<EleccionLider> {
        let protocolo = ProtocoloLider::new(DNS::direccion_lider(&id))?;

        let mut ret = EleccionLider {
            id,
            protocolo,
            id_lider: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            obtuve_ok: Arc::new((Mutex::new(false), Condvar::new())),
            stop: Arc::new(AtomicBool::new(false)),
            respondedor: None,
        };

        let hay_lider = ret.buscar_si_existe_lider();

        let mut threads = Vec::new();

        match hay_lider {
            Some(id_lider) => {
                ret.set_id_lider(Some(id_lider), true);
                let mut clone = ret.clone();
                threads.push(thread::spawn(move || clone.mantener_vivo()));
                let mut clone = ret.clone();
                ret.respondedor = Some(thread::spawn(move || clone.responder(threads)));
            },
            None => {
                let mut clone = ret.clone();
                ret.respondedor = Some(thread::spawn(move || clone.responder(threads)));
                ret.buscar_nuevo_lider()
            }
        }

        Ok(ret)
    }

    fn buscar_si_existe_lider(&mut self) -> Option<usize> {
        println!("Envio a {:?} un VERIFICAR", (0..TEAM_MEMBERS));
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                let _ = self.enviar(CodigoLider::VERIFICAR, id);
            }
        });

        println!("Procedo a la recibicion de mensajes");
        let mut duracion = 2000;
        loop {
            duracion -= 200;
            if duracion < 1000 { break }
            
            match self.protocolo.recibir(Some(Duration::from_millis(duracion))) {
                Ok(mensaje) => {
                    println!("Recibí {:?}", mensaje.codigo);
                    if let CodigoLider::OK = mensaje.codigo {
                        return Some(mensaje.id_emisor);
                    }
                }
                Err(_) => break
            };
        }

        None
    }

    // TODO: Documentacion
    pub fn bloquear_si_no_soy_lider(&self) -> bool {
        let _ = self.id_lider
            .1
            .wait_while(self.id_lider.0.lock()
            .expect("Error al tomar lock del id_lider en EleccionLider"), |id_lider| {
                if let Some(id) = *id_lider {  id != self.id } else { false }
            }).expect("Error al tomar lock del id_lider en EleccionLider");

        true
    }

    // TODO: Documentacion
    pub fn soy_lider(&self) -> bool {
        self.get_id_lider() == self.id
    }

    // TODO: Documentacion
    pub fn get_id_lider(&self) -> usize {    
        self.id_lider
            .1
            .wait_while(self.id_lider.0.lock()
            .expect("Error al tomar lock del id_lider en EleccionLider"), |id_lider| {
                id_lider.is_none()
            }).expect("Error al tomar lock del id_lider en EleccionLider")
            .expect("Se obtuvo un id None")
    }

    // TODO: Documentacion
    pub fn notificar_finalizacion(&mut self) {
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                let _ = self.enviar(CodigoLider::ELECCION, id);
            }
        });
    }

    // TODO: Documentacion
    pub fn buscar_nuevo_lider(&mut self) {
        if self.stop.load(Ordering::Relaxed) { return; }
        
        match self.id_lider.0.lock() {
            Ok(mut lider) => {
                if lider.is_none() { return; } // Ya se esta buscando lider
                else { *lider = None }
            },
            Err(_) => panic!("Error al tomar el lock de id_lider en EleccionLider")
        }
        
        *self.obtuve_ok.0.lock().expect("Error al tomar el lock de obtuve_ok en EleccionLider") = false;

        self.enviar_eleccion();
        let obtuve_ok = self.obtuve_ok.1.wait_timeout_while(
            self.obtuve_ok.0.lock().expect("Error al tomar lock de obtuve_ok en EleccionLider"),
            TIMEOUT_LIDER,
            |got_it| !*got_it,
        );

        //Si rompe, poner esto
        if !*obtuve_ok.expect("Error al tomar el lock de obtuve_ok en EleccionLider").0 {
            self.anunciarme_lider()
        } else {
            let _ = self.get_id_lider();
        }
    }

    // TODO: Documentacion
    pub fn finalizar(&mut self) {
        //*self.stop.0.lock().expect("Error al tomar lock de stop en EleccionLider") = true;
        self.stop.store(true, Ordering::Relaxed);
        self.notificar_finalizacion();
        if let Some(res) = self.respondedor.take() {
            let _ = res.join();
        }
    }

    ////////////////////////////////////////////////////////////////////
    //                                                                //
    //                     FUNCIONES PRIVADAS                         //
    //                                                                //
    ////////////////////////////////////////////////////////////////////

    // TODO: Documentacion a todas las de abajo?? Son privadas
    fn enviar(&mut self, codigo: CodigoLider, id_destino: usize) -> Resultado<()> {
        let mensaje = MensajeLider::new(codigo, self.id);
        self.protocolo
            .enviar(&mensaje, DNS::direccion_lider(&id_destino))
    }

    fn enviar_eleccion(&mut self) {
        thread::sleep(Duration::from_millis(500)); // TODO: CAMBIAR ESTO
        ((self.id + 1)..TEAM_MEMBERS)
            .for_each(|id| {
                let _ = self.enviar(CodigoLider::ELECCION, id);
            });
    }

    fn anunciarme_lider(&mut self) {
        println!("[Eleccion]: Me anuncio como lider");
        
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                let _ = self.enviar(CodigoLider::COORDINADOR, id);
            }
        });
        
        self.set_id_lider(Some(self.id), true);
    }

    fn set_id_lider(&mut self, val: Option<usize>, notificar: bool) {
        *self.id_lider.0.lock().expect("Error al tomar el lock de id_lider en EleccionLider") = val;
        if notificar {self.id_lider.1.notify_all();}
    }

    fn responder(&mut self, mut threads: Vec<JoinHandle<()>>) {
        while !self.stop.load(Ordering::Relaxed) { //TODO: Cambiar a AtomicBool
            // TODO: revisar el timeout
            if let Ok(mensaje) = self.protocolo.recibir(Some(TIMEOUT_MENSAJE)) {
                let id_emisor = mensaje.id_emisor;
                match mensaje.codigo {
                    CodigoLider::OK => self.recibir_ok(),
                    CodigoLider::ELECCION => self.recibir_election(&mut threads, id_emisor),
                    CodigoLider::COORDINADOR => self.recibir_coordinador(&mut threads, id_emisor),
                    CodigoLider::VERIFICAR => self.recibir_verificar(id_emisor),
                };
            } else {
                // Hubo timeout, por lo tanto no recibí nada
                let mut me = self.clone();
                if !self.soy_lider() {
                    // TODO: Posible Deadlock
                    threads.push(thread::spawn(move || me.buscar_nuevo_lider()));
                    // TODO: revisar esto
                }
            }
        }

        let _ = threads.into_iter().map(|t| t.join());
    }

    fn recibir_ok(&mut self) {
        *self
            .obtuve_ok
            .0
            .lock()
            .expect("[Eleccion Lider]: Error al intentar tomar el lock de ok") = true;
        self.obtuve_ok.1.notify_all();
    }

    fn recibir_election(&mut self, threads: &mut Vec<JoinHandle<()>>, id_emisor: usize) {
        println!("[Eleccion {}] recibí ELECCION de {}", self.id, id_emisor);
        if id_emisor < self.id {
            let _ = self.enviar(CodigoLider::OK, id_emisor);

            let mut me = self.clone();
            threads.push(thread::spawn(move || me.buscar_nuevo_lider())); // TODO: revisar esto
        }
    }

    fn recibir_coordinador(&mut self, threads: &mut Vec<JoinHandle<()>>, id_emisor: usize) {
        println!("[Eleccion]: El nuevo lider es {}", id_emisor);

        self.set_id_lider(Some(id_emisor), true);
        let mut me = self.clone();
        threads.push(thread::spawn(move || me.mantener_vivo())); // TODO: revisar esto
    }

    fn recibir_verificar(&mut self, id_emisor: usize) {
        if self.soy_lider() {
            let _ = self.enviar(CodigoLider::OK, id_emisor);
        }
    }

    fn mantener_vivo(&mut self) {
        while !self.soy_lider() {
            println!("[Eleccion]: Envío VERIFICAR al lider de ID {}", self.get_id_lider());
            if self
                .enviar(CodigoLider::VERIFICAR, self.get_id_lider())
                .is_ok()
            {
                thread::sleep(TIMEOUT_MANTENER_VIVO);
            }
        }
    }

    fn clone(&self) -> EleccionLider {
        EleccionLider {
            id: self.id,
            protocolo: self.protocolo.clone(), // TODO: ACA LE SAQUE EL UNWRAP
            id_lider: self.id_lider.clone(),
            obtuve_ok: self.obtuve_ok.clone(),
            stop: self.stop.clone(),
            respondedor: None,
        }
    }
}
