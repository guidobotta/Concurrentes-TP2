use std::collections::{HashMap};
use std::sync::{Arc, Condvar, Mutex};
use std::{thread};
use std::time::Duration;
use common::error::{ErrorApp, ErrorInterno, Resultado};
use common::protocolo::Protocolo;
use super::pago::Pago;
use common::mensaje::{Mensaje, CodigoMensaje};

fn id_to_addr(id: usize) -> String { "127.0.0.1:1234".to_owned() + &*id.to_string() }

const STAKEHOLDERS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(10);
const TRANSACTION_COORDINATOR_ADDR: &str = "127.0.0.1:1234";

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u32);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum EstadoTransaccion {
    Wait,
    Commit,
    Abort,
}

struct CoordinadorTransaccion {
    log: HashMap<usize, EstadoTransaccion>,
    protocolo: Protocolo,
    responses: Arc<(Mutex<Vec<Option<Mensaje>>>, Condvar)>,
    id: usize,
    destinatarios: Vec<String>
}

fn direccion_desde_id(id: &usize) -> String {
    format!("127.0.0.1:500{}", *id) // TODO: Mejorar
}

impl CoordinadorTransaccion {
    fn new(id: usize) -> Self {
        let mut ret = CoordinadorTransaccion {
            log: HashMap::new(),
            protocolo: Protocolo::new(TRANSACTION_COORDINATOR_ADDR.to_string()).unwrap(),
            responses: Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new())),
            id,
            destinatarios: vec![0, 1, 2].iter().map(direccion_desde_id).collect() // TODO: cambiar esto
        };

        thread::spawn(move || CoordinadorTransaccion::responder(ret.protocolo, ret.responses));

        ret
    }

    fn submit(&mut self, pago: Pago) {
        match self.log.get(&pago.get_id()) {
            None => self.full_protocol(&pago),
            Some(EstadoTransaccion::Wait) => self.full_protocol(&pago),
            Some(EstadoTransaccion::Commit) => { self.commit(&pago); },
            Some(EstadoTransaccion::Abort) => { self.abort(&pago); }
        }
    }

    fn full_protocol(&mut self, pago: &Pago) {
        match self.prepare(pago) {
            Ok(_) => { self.commit(pago); }, // TODO: ver que hacer con el result de estos (quizas reintentar commit)
            Err(_) => { self.abort(pago); } // TODO: ver que hacer con el result de estos y tema de escribir para reintentar
        }
    }

    fn prepare(&mut self, pago: &Pago) -> Resultado<()> {
        self.log.insert(pago.get_id(), EstadoTransaccion::Wait);
        println!("[COORDINATOR] prepare {}", pago.get_id());

        let id_op = pago.get_id();

        // Preparo los mensajes a enviar
        let m_hotel = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_hotel()}, self.id, id_op);
        let m_aerolinea = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_aerolinea()}, self.id, id_op);
        let m_banco = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_hotel() + pago.get_monto_aerolinea()}, self.id, id_op); // TODO: PASAR ESTO AL PARSER 

        // Mensaje esperado
        let esperado = Mensaje::new(CodigoMensaje::READY, self.id, id_op); // TODO: PASAR ESTO AL PARSER 
        
        self.send_and_wait(vec![m_hotel, m_aerolinea, m_banco], esperado)
    }

    fn commit(&mut self, pago: &Pago) -> Resultado<()> {
        self.log.insert(pago.get_id(), EstadoTransaccion::Commit);
        println!("[COORDINATOR] commit {}", pago.get_id());

        let id_op = pago.get_id();

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = Mensaje::new(CodigoMensaje::COMMIT, self.id, id_op); // TODO: pensar si hacer que esperado sea finished o no

        self.send_and_wait(vec![mensaje, mensaje, mensaje], mensaje)
    }

    fn abort(&mut self, pago: &Pago) -> Resultado<()> {
        self.log.insert(pago.get_id(), EstadoTransaccion::Abort);
        println!("[COORDINATOR] abort {}", pago.get_id());

        let id_op = pago.get_id();

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = Mensaje::new(CodigoMensaje::ABORT, self.id, id_op);

        self.send_and_wait(vec![mensaje, mensaje, mensaje], mensaje)
    }

    //Queremos que se encargue de enviarle los mensajes a los destinatarios
    //y espere por sus respuestas, con un timeout y numero de intentos dado
    fn send_and_wait(&self, 
                     mensajes: Vec<Mensaje>, 
                     esperado: Mensaje) -> Resultado<()> {

        let mut responses;
        *self.responses.0.lock().unwrap() = vec![None; STAKEHOLDERS];

        loop {
            for (idx, mensaje) in mensajes.iter().enumerate() {
                self.protocolo.enviar(&mensaje, self.destinatarios[idx]).unwrap();
            }
            responses = self.responses.1.wait_timeout_while(self.responses.0.lock().unwrap(), TIMEOUT, |responses| responses.iter().any(Option::is_none));

            if responses.is_ok() { break; }
            println!("[COORDINATOR] timeout {}", esperado.id_op);
        }

        if !responses.unwrap().0.iter().all(|opt| opt.is_some() && opt.unwrap() == esperado) {
            return Err(ErrorApp::Interno(ErrorInterno::new("Respuesta no esperada")));
        }

        Ok(())
    }

    fn responder(protocolo: Protocolo, responses: Arc<(Mutex<Vec<Option<Mensaje>>>, Condvar)>) {
        loop {
            let mensaje = protocolo.recibir(None).unwrap(); // TODO: revisar el timeout
            let id_emisor = mensaje.id_emisor;

            match mensaje.codigo {
                CodigoMensaje::READY => {
                    println!("[COORDINATOR] recibí COMMIT de {}", id_emisor);
                    responses.0.lock().unwrap()[id_emisor] = Some(mensaje);
                    responses.1.notify_all();
                }
                CodigoMensaje::ABORT => {
                    println!("[COORDINATOR] recibí ABORT de {}", id_emisor);
                    responses.0.lock().unwrap()[id_emisor] = Some(mensaje);
                    responses.1.notify_all();
                }
                _ => {
                    println!("[COORDINATOR] recibí algo que no puedo interpretar {}", id_emisor);
                }
            }
        }
    }
}
