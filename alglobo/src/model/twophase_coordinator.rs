use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::mem::size_of;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, io};
use std::time::Duration;
use super::error::{ErrorApp, ErrorInterno, Resultado};
use super::protocolo::Protocolo;
use super::pago::{self, Pago};

use rand::{Rng, thread_rng};
use std::convert::TryInto;
use crate::TransactionState::{Wait, Commit};
use crate::model::protocolo::Mensaje;

fn id_to_addr(id: usize) -> String { "127.0.0.1:1234".to_owned() + &*id.to_string() }

const STAKEHOLDERS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(10);
const TRANSACTION_COORDINATOR_ADDR: &str = "127.0.0.1:1234";

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u32);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum TransactionState {
    Wait,
    Commit,
    Abort,
}

struct TransactionCoordinator {
    log: HashMap<usize, TransactionState>,
    protocolo: Protocolo,
    responses: Arc<(Mutex<Vec<Option<Mensaje>>>, Condvar)>,
}


impl TransactionCoordinator {
    fn new() -> Self {
        let mut ret = TransactionCoordinator {
            log: HashMap::new(),
            protocolo: Protocolo::new(TRANSACTION_COORDINATOR_ADDR.to_string()).unwrap(),
            responses: Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new())),
        };

        let mut clone = ret.clone();
        thread::spawn(move || clone.responder());

        ret
    }

    fn submit(&mut self, pago: Pago) -> bool {
        match self.log.get(&pago.get_id()) {
            None => self.full_protocol(&pago, false),
            Some(TransactionState::Wait) => self.full_protocol(&pago, true),
            Some(TransactionState::Commit) => self.commit(&pago),
            Some(TransactionState::Abort) => self.abort(&pago)
        }
    }

    fn full_protocol(&mut self, pago: &Pago, force_continue: bool) -> bool {
        if self.prepare(pago) {
            self.commit(pago)
        } else {
            self.abort(pago)
        }
    }

    fn prepare(&mut self, pago: &Pago) -> bool {
        self.log.insert(t, TransactionState::Wait);
        println!("[COORDINATOR] prepare {}", pago.get_id());
        let m_hotel = Mensaje::PREPARE {id: pago.get_id(), monto: pago.get_monto_hotel()};
        let m_aerolinea = Mensaje::PREPARE {id: pago.get_id(), monto: pago.get_monto_aerolinea()};
        let m_banco = Mensaje::PREPARE {id: pago.get_id(), monto: pago.get_monto_hotel() + pago.get_monto_aerolinea()}; // TODO: PASAR ESTO AL PARSER 

        let esperado = Mensaje::COMMIT { id: pago.get_id() };
        
        
        self.send_and_wait(vec![m_hotel, m_aerolinea, m_banco], destinatarios, retries, esperado);

        true
    }

    fn commit(&mut self, pago: &Pago) -> bool {
        self.log.insert(t, TransactionState::Commit);
        println!("[COORDINATOR] commit {}", t.0);
        self.broadcast_and_wait(b'C', t, TransactionState::Commit)
    }

    fn abort(&mut self, pago: &Pago) -> bool {
        self.log.insert(t, TransactionState::Abort);
        println!("[COORDINATOR] abort {}", t.0);
        !self.broadcast_and_wait(b'A', t, TransactionState::Abort)
    }

    //Queremos que se encargue de enviarle los mensajes a los destinatarios
    //y espere por sus respuestas, con un timeout y numero de intentos dado
    fn send_and_wait(&self, 
                     mensajes: Vec<Mensaje>, 
                     destinatarios: Vec<String>,
                     retries: Option<usize>,
                     esperado: Mensaje) -> Resultado<()> {

        let mut responses;

        loop {
            for (idx, mensaje) in mensajes.iter().enumerate() {
                self.protocolo.enviar(&mensaje, destinatarios[idx]).unwrap();
            }
            responses = self.responses.1.wait_timeout_while(self.responses.0.lock().unwrap(), TIMEOUT, |responses| responses.iter().any(Option::is_none));

            if responses.is_ok() { break; }

            if let Some(ref mut val) = retries {
                *val -= 1;
                if *val <= 0 { break; }
            }
        }

        if !responses.unwrap().0.iter().all(|opt| opt.is_some() && opt.unwrap() == esperado) {
            return Err(ErrorApp::Interno(ErrorInterno::new("Respuesta no esperada")));
        }

        Ok(())
    }


    fn broadcast_and_wait(&self, message: u8, t: TransactionId, expected: TransactionState) -> bool {
        *self.responses.0.lock().unwrap() = vec![None; STAKEHOLDERS];
        let mut msg = vec!(message);
        msg.extend_from_slice(&t.0.to_le_bytes());
        for stakeholder in 0..STAKEHOLDERS {
            println!("[COORDINATOR] envio {} id {} a {}", message, t.0, stakeholder);
            self.socket.send_to(&msg, id_to_addr(stakeholder)).unwrap();
        }
        let responses = self.responses.1.wait_timeout_while(self.responses.0.lock().unwrap(), TIMEOUT, |responses| responses.iter().any(Option::is_none));
        if responses.is_err() {
            println!("[COORDINATOR] timeout {}", t.0);
            false
        } else {
            responses.unwrap().0.iter().all(|opt| opt.is_some() && opt.unwrap() == expected)
        }
    }

    fn responder(&mut self) {
        loop {
            let mut buf = [0; size_of::<usize>() + 1];
            let (size, from) = self.socket.recv_from(&mut buf).unwrap();
            let id_from = usize::from_le_bytes(buf[1..].try_into().unwrap());

            match &buf[0] {
                b'C' => {
                    println!("[COORDINATOR] recibí COMMIT de {}", id_from);
                    self.responses.0.lock().unwrap()[id_from] = Some(TransactionState::Commit);
                    self.responses.1.notify_all();
                }
                b'A' => {
                    println!("[COORDINATOR] recibí ABORT de {}", id_from);
                    self.responses.0.lock().unwrap()[id_from] = Some(TransactionState::Abort);
                    self.responses.1.notify_all();
                }
                _ => {
                    println!("[COORDINATOR] ??? {}", id_from);
                }
            }
        }
    }

    fn clone(&self) -> Self {
        TransactionCoordinator {
            log: HashMap::new(),
            socket: self.socket.try_clone().unwrap(),
            responses: self.responses.clone(),
        }
    }
}
