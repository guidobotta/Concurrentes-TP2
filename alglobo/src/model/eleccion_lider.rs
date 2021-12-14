use common::dns::DNS;
use common::error::Resultado;
use common::protocolo_lider::{CodigoLider, MensajeLider, ProtocoloLider};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const TEAM_MEMBERS: usize = 5;
const TIMEOUT_LIDER: Duration = Duration::from_secs(6); // <- Si pasa este tiempo me hago lider
const TIMEOUT_MENSAJE: Duration = Duration::from_secs(10); // <- Tolerancia a recibir un mensaje
const TIMEOUT_MANTENER_VIVO: Duration = Duration::from_secs(2); // <- Frecuencia de enviado del keep alive
const ID_LIDER_DEFAULT: usize = 0;

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
            id_lider: Arc::new((Mutex::new(Some(ID_LIDER_DEFAULT)), Condvar::new())), //El id default de lider
            obtuve_ok: Arc::new((Mutex::new(false), Condvar::new())),
            stop: Arc::new(AtomicBool::new(false)),
            respondedor: None,
        };

        ret.inicializar();

        Ok(ret)
    }

    fn inicializar(&mut self) {
        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                let _ = self.enviar(CodigoLider::VERIFICAR, id);
            }
        });

        let mut threads = Vec::new();
        let mut clone = self.clone();
        threads.push(thread::spawn(move || clone.mantener_vivo()));
        let mut clone = self.clone();
        self.respondedor = Some(thread::spawn(move || clone.responder(threads)));
    }

    /// Bloquea si nodo no es lider
    pub fn bloquear_si_no_soy_lider(&self) -> bool {
        let _e = self
            .id_lider
            .1
            .wait_while(
                self.id_lider
                    .0
                    .lock()
                    .expect("Error al tomar lock del id_lider en EleccionLider"),
                |id_lider| {
                    if let Some(id) = *id_lider {
                        id != self.id
                    } else {
                        true
                    } //TODO: Creo que el else deberia ser true
                },
            )
            .expect("Error al tomar lock del id_lider en EleccionLider");

        true
    }

    /// Devuelve true si el proceso es lider. Es bloqueante
    pub fn soy_lider(&self) -> bool {
        self.get_id_lider() == self.id
    }

    /// Devuelve el id del proceso lider. Es bloquean
    pub fn get_id_lider(&self) -> usize {
        self.id_lider
            .1
            .wait_while(
                self.id_lider
                    .0
                    .lock()
                    .expect("Error al tomar lock del id_lider en EleccionLider"),
                |id_lider| id_lider.is_none(),
            )
            .expect("Error al tomar lock del id_lider en EleccionLider")
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

    /// Comienza la busqueda de un nuevo lider
    pub fn buscar_nuevo_lider(&mut self) {
        if self.stop.load(Ordering::Relaxed) {
            return;
        }

        match self.id_lider.0.lock() {
            Ok(mut lider) => {
                if lider.is_none() {
                    return;
                }
                // Ya se esta buscando lider
                else {
                    *lider = None
                }
            }
            Err(_) => panic!("Error al tomar el lock de id_lider en EleccionLider"),
        }

        *self
            .obtuve_ok
            .0
            .lock()
            .expect("Error al tomar el lock de obtuve_ok en EleccionLider") = false;

        self.enviar_eleccion();
        let obtuve_ok = self.obtuve_ok.1.wait_timeout_while(
            self.obtuve_ok
                .0
                .lock()
                .expect("Error al tomar lock de obtuve_ok en EleccionLider"),
            TIMEOUT_LIDER,
            |got_it| !*got_it,
        );

        //Si rompe, poner esto
        if !*obtuve_ok
            .expect("Error al tomar el lock de obtuve_ok en EleccionLider")
            .0
        {
            self.anunciarme_lider()
        } else {
            let _ = self.get_id_lider();
        }
    }

    /// Finaliza ordenadamente
    pub fn finalizar(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.id_lider.1.notify_all();
        self.obtuve_ok.1.notify_all();
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

    /// Enviar mensaje al nodo de id_destino
    fn enviar(&mut self, codigo: CodigoLider, id_destino: usize) -> Resultado<()> {
        let mensaje = MensajeLider::new(codigo, self.id);
        self.protocolo
            .enviar(&mensaje, DNS::direccion_lider(&id_destino))
    }

    /// Envia eleccion a los nodos de id mayor
    fn enviar_eleccion(&mut self) {
        ((self.id + 1)..TEAM_MEMBERS).for_each(|id| {
            let _ = self.enviar(CodigoLider::ELECCION, id);
        });
    }

    /// Envia coordinador a todos los nodos
    fn anunciarme_lider(&mut self) {
        println!("[Eleccion]: Me anuncio como lider");

        (0..TEAM_MEMBERS).for_each(|id| {
            if id != self.id {
                let _ = self.enviar(CodigoLider::COORDINADOR, id);
            }
        });

        self.set_id_lider(Some(self.id), true);
    }

    /// Setea id lider
    fn set_id_lider(&mut self, val: Option<usize>, notificar: bool) {
        *self
            .id_lider
            .0
            .lock()
            .expect("Error al tomar el lock de id_lider en EleccionLider") = val;
        if notificar {
            self.id_lider.1.notify_all();
        }
    }

    /// Recibe mensajes de otros nodos y los procesa
    fn responder(&mut self, mut threads: Vec<JoinHandle<()>>) {
        while !self.stop.load(Ordering::Relaxed) {
            if let Ok(mensaje) = self.protocolo.recibir(Some(TIMEOUT_MENSAJE)) {
                let id_emisor = mensaje.id_emisor;
                match mensaje.codigo {
                    CodigoLider::OK => self.recibir_ok(),
                    CodigoLider::ELECCION => self.recibir_election(&mut threads, id_emisor),
                    CodigoLider::COORDINADOR => self.recibir_coordinador(id_emisor),
                    CodigoLider::VERIFICAR => self.recibir_verificar(id_emisor),
                };
            } else {
                // Hubo timeout, por lo tanto no recibí nada
                let mut me = self.clone();
                if !self.soy_lider() {
                    threads.push(thread::spawn(move || me.buscar_nuevo_lider()));
                }
            }
        }

        let _ = threads.into_iter().map(|t| t.join());
    }

    /// Procesa un mensaje ok
    fn recibir_ok(&mut self) {
        *self
            .obtuve_ok
            .0
            .lock()
            .expect("[Eleccion Lider]: Error al intentar tomar el lock de ok") = true;
        self.obtuve_ok.1.notify_all();
    }

    /// Procesa un mensaje eleccion
    fn recibir_election(&mut self, threads: &mut Vec<JoinHandle<()>>, id_emisor: usize) {
        println!("[Eleccion {}] recibí ELECCION de {}", self.id, id_emisor);
        let _ = self.enviar(CodigoLider::OK, id_emisor);
        let mut me = self.clone();
        if self
            .id_lider
            .0
            .lock()
            .expect("Error al tomar el lock de id_lider en EleccionLider")
            .is_some()
        {
            threads.push(thread::spawn(move || me.buscar_nuevo_lider()));
        }
    }

    /// Procesa un mensaje coordinador
    fn recibir_coordinador(&mut self, id_emisor: usize) {
        println!("[Eleccion]: El lider es {}", id_emisor);
        self.set_id_lider(Some(id_emisor), true);
    }

    /// Procesa un mensaje verificar
    fn recibir_verificar(&mut self, id_emisor: usize) {
        if self.soy_lider() {
            let _ = self.enviar(CodigoLider::COORDINADOR, id_emisor);
        }
    }

    fn esperar_mientras_sea_lider(&mut self) {
        let _e = self
            .id_lider
            .1
            .wait_while(
                self.id_lider
                    .0
                    .lock()
                    .expect("Error al tomar lock del id_lider en EleccionLider"),
                |id_lider| {
                    let continuar = !self.stop.load(Ordering::Relaxed);
                    if let Some(id) = *id_lider {
                        id == self.id && continuar
                    } else {
                        continuar
                    }
                },
            )
            .expect("Error al tomar lock del id_lider en EleccionLider");
    }

    /// Envia mensaje VERIFICAR al lider actual
    fn mantener_vivo(&mut self) {
        loop {
            self.esperar_mientras_sea_lider();
            if self.stop.load(Ordering::Relaxed) {
                return;
            }

            println!(
                "[Eleccion]: Envío VERIFICAR al lider de ID {}",
                self.get_id_lider()
            );
            let _ = self.enviar(CodigoLider::VERIFICAR, self.get_id_lider());
            thread::sleep(TIMEOUT_MANTENER_VIVO);
        }
    }

    /// Devuelve una copia de EleccionLider
    fn clone(&self) -> EleccionLider {
        EleccionLider {
            id: self.id,
            protocolo: self.protocolo.clone(),
            id_lider: self.id_lider.clone(),
            obtuve_ok: self.obtuve_ok.clone(),
            stop: self.stop.clone(),
            respondedor: None,
        }
    }
}
