use common::dns::DNS;
use common::error::Resultado;
use common::protocolo_lider::{CodigoLider, MensajeLider, ProtocoloLider};
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
    stop: Arc<(Mutex<bool>, Condvar)>,
    respondedor: Option<JoinHandle<()>>,
}

impl EleccionLider {
    /// Devuelve una instancia de EleccionLider.
    /// Recibe el id asociado al nodo de alglobo.
    pub fn new(id: usize) -> EleccionLider {
        let protocolo = ProtocoloLider::new(DNS::direccion_lider(&id)).unwrap();

        let mut ret = EleccionLider {
            id,
            protocolo,
            id_lider: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            obtuve_ok: Arc::new((Mutex::new(false), Condvar::new())),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            respondedor: None,
        };

        let mut clone = ret.clone();
        ret.respondedor = Some(thread::spawn(move || clone.responder()));

        ret.buscar_nuevo_lider();
        ret
    }

    // TODO: Documentacion
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

    // TODO: Documentacion
    pub fn soy_lider(&self) -> bool {
        self.get_id_lider() == self.id
    }

    // TODO: Documentacion
    pub fn get_id_lider(&self) -> usize {
        self.id_lider
            .1
            .wait_while(self.id_lider.0.lock().unwrap(), |id_lider| {
                id_lider.is_none()
            })
            .unwrap()
            .unwrap()
    }

    // TODO: Documentacion
    pub fn notificar_finalizacion(&mut self) {
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                self.enviar(CodigoLider::ELECCION, id).unwrap()
            }
        });
    }

    // TODO: Documentacion
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
        let obtuve_ok = self.obtuve_ok.1.wait_timeout_while(
            self.obtuve_ok.0.lock().unwrap(),
            TIMEOUT_LIDER,
            |got_it| !*got_it,
        );

        if !*obtuve_ok.unwrap().0 {
            self.anunciarme_lider()
        } else {
            let _ = self
                .id_lider
                .1
                .wait_while(self.id_lider.0.lock().unwrap(), |id_lider| {
                    id_lider.is_none()
                });
        }
    }

    // TODO: Documentacion
    pub fn finalizar(&mut self) {
        *self.stop.0.lock().unwrap() = true;
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
            .for_each(|id| self.enviar(CodigoLider::ELECCION, id).unwrap());
    }

    fn anunciarme_lider(&mut self) {
        println!("[ELECCION]: Me anuncio como lider");
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                self.enviar(CodigoLider::COORDINADOR, id).unwrap()
            }
        });

        *self.id_lider.0.lock().unwrap() = Some(self.id);
        self.id_lider.1.notify_all();
    }

    fn responder(&mut self) {
        let mut threads = Vec::new();
        while !*self.stop.0.lock().unwrap() {
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
        println!("[ELECCION {}] recibí ELECCION de {}", self.id, id_emisor);
        if id_emisor < self.id {
            self.enviar(CodigoLider::OK, id_emisor).unwrap();

            let mut me = self.clone();
            threads.push(thread::spawn(move || me.buscar_nuevo_lider())); // TODO: revisar esto
        }
    }

    fn recibir_coordinador(&mut self, threads: &mut Vec<JoinHandle<()>>, id_emisor: usize) {
        println!("[ELECCION]: El nuevo lider es {}", id_emisor);
        *self.id_lider.0.lock().unwrap() = Some(id_emisor);
        self.id_lider.1.notify_all();

        let mut me = self.clone();
        threads.push(thread::spawn(move || me.mantener_vivo())); // TODO: revisar esto
    }

    fn recibir_verificar(&mut self, id_emisor: usize) {
        if self.soy_lider() {
            self.enviar(CodigoLider::OK, id_emisor).unwrap()
        }
    }

    fn mantener_vivo(&mut self) {
        while !self.soy_lider() {
            if self
                .enviar(CodigoLider::VERIFICAR, self.get_id_lider())
                .is_ok()
            {
                thread::sleep(TIMEOUT_MANTENER_VIVO);
            };
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
