use std::mem::size_of;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use common::dns::DNS;
use std::convert::TryInto;

const TEAM_MEMBERS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct EleccionLider {
    id: usize,
    socket: UdpSocket,
    leader_id: Arc<(Mutex<Option<usize>>, Condvar)>,
    got_ok: Arc<(Mutex<bool>, Condvar)>,
    stop: Arc<(Mutex<bool>, Condvar)>,
}

impl EleccionLider {
    pub fn new(id: usize) -> EleccionLider {
        let mut ret = EleccionLider {
            id,
            socket: UdpSocket::bind(DNS::direccion_lider(&id)).unwrap(),
            leader_id: Arc::new((Mutex::new(Some(id)), Condvar::new())),
            got_ok: Arc::new((Mutex::new(false), Condvar::new())),
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
        self.leader_id
            .1
            .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                leader_id.is_none()
            })
            .unwrap()
            .unwrap()
    }

    pub fn buscar_nuevo_lider(&mut self) {
        if *self.stop.0.lock().unwrap() {
            return;
        }

        if self.leader_id.0.lock().unwrap().is_none() {
            // ya esta buscando lider
            return;
        }

        println!("[{}] buscando lider", self.id);

        *self.got_ok.0.lock().unwrap() = false;
        *self.leader_id.0.lock().unwrap() = None;
        self.enviar_eleccion();
        let got_ok =
            self.got_ok
                .1
                .wait_timeout_while(self.got_ok.0.lock().unwrap(), TIMEOUT, |got_it| !*got_it);
                
        if !*got_ok.unwrap().0 {
            self.anunciarme_lider()
        } else {
            self.leader_id
                .1
                .wait_while(self.leader_id.0.lock().unwrap(), |leader_id| {
                    leader_id.is_none()
                });
        }
    }

    fn id_a_mensaje(&self, header: u8) -> Vec<u8> {
        let mut msg = vec![header];
        msg.extend_from_slice(&self.id.to_le_bytes());
        msg
    }

    fn enviar_eleccion(&self) {
        // P envía el mensaje ELECTION a todos los procesos que tengan número mayor
        let msg = self.id_a_mensaje(b'E');
        for peer_id in (self.id + 1)..TEAM_MEMBERS {
            self.socket.send_to(&msg, DNS::direccion_lider(&peer_id)).unwrap();
        }
    }

    fn anunciarme_lider(&self) {
        // El nuevo coordinador se anuncia con un mensaje COORDINATOR
        println!("[{}] me anuncio como lider", self.id);
        let msg = self.id_a_mensaje(b'C');

        for peer_id in 0..TEAM_MEMBERS {
            if peer_id != self.id {
                self.socket.send_to(&msg, DNS::direccion_lider(&peer_id)).unwrap();
            }
        }
        
        *self.leader_id.0.lock().unwrap() = Some(self.id);
    }

    fn responder(&mut self) {
        while !*self.stop.0.lock().unwrap() {
            let mut buf = [0; size_of::<usize>() + 1];
            let (size, from) = self.socket.recv_from(&mut buf).unwrap();
            let id_from = usize::from_le_bytes(buf[1..].try_into().unwrap());
            if *self.stop.0.lock().unwrap() {
                break;
            }

            match &buf[0] {
                b'O' => {
                    println!("[{}] recibí OK de {}", self.id, id_from);
                    *self.got_ok.0.lock().unwrap() = true;
                    self.got_ok.1.notify_all();
                }
                b'E' => {
                    println!("[{}] recibí Election de {}", self.id, id_from);
                    if id_from < self.id {
                        self.socket
                            .send_to(&self.id_a_mensaje(b'O'), DNS::direccion_lider(&id_from))
                            .unwrap();
                        let mut me = self.clone();
                        thread::spawn(move || me.buscar_nuevo_lider());
                    }
                }
                b'C' => {
                    println!("[{}] recibí nuevo coordinador {}", self.id, id_from);
                    *self.leader_id.0.lock().unwrap() = Some(id_from);
                    self.leader_id.1.notify_all();
                }
                _ => {
                    println!("[{}] ??? {}", self.id, id_from);
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
            socket: self.socket.try_clone().unwrap(),
            leader_id: self.leader_id.clone(),
            got_ok: self.got_ok.clone(),
            stop: self.stop.clone(),
        }
    }
}
