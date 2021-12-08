use std::sync::{Arc, Condvar, Mutex};
use std::{thread};
use std::time::Duration;
use common::error::{ErrorApp, ErrorInterno, Resultado};
use common::protocolo::Protocolo;
use super::log::{Log, Transaccion, EstadoTransaccion};
use super::pago::Pago;
use common::mensaje::{Mensaje, CodigoMensaje};
use common::dns::DNS;

const STAKEHOLDERS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u32);


pub struct CoordinadorTransaccion {
    log: Log,
    protocolo: Protocolo,
    responses: Arc<(Mutex<Vec<Option<Mensaje>>>, Condvar)>,
    id: usize,
    destinatarios: Vec<String>,
}

impl CoordinadorTransaccion {
    pub fn new(id: usize, log: Log) -> Self {

        let protocolo = Protocolo::new(DNS::direccion_alglobo(&id)).unwrap();
        let responses =  Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new()));
        let ret = CoordinadorTransaccion {
            log,
            protocolo: protocolo.clone(),
            responses: responses.clone(),
            id,
            destinatarios: vec![0, 1, 2].iter().map(|id| DNS::direccion_webservice(&id)).collect() // TODO: cambiar esto
        };

        thread::spawn(move || CoordinadorTransaccion::responder(protocolo, responses));

        ret
    }

    pub fn submit(&mut self, transaccion: &mut Transaccion) -> Resultado<()>{
        match self.log.obtener(&transaccion.id) {
            None => self.full_protocol(transaccion),
            Some(t) => match t.estado {
                EstadoTransaccion::Prepare => self.full_protocol(transaccion),
                EstadoTransaccion::Commit => { self.commit(transaccion) },
                EstadoTransaccion::Abort => { 
                    let _ = self.abort(transaccion);
                    return Err(ErrorApp::Interno(ErrorInterno::new("Transaccion abortada")));
                }
            }
        }
    }

    fn full_protocol(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        match self.prepare(transaccion) {
            Ok(_) => { self.commit(transaccion) }, // TODO: ver que hacer con el result de estos (quizas reintentar commit)
            Err(e) => { 
                let _ = self.abort(transaccion); // TODO: ver que hacer con el result de estos y tema de escribir para reintentar
                Err(e)
             } 
        }
    }

    fn prepare(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log.insertar(transaccion.prepare());
        println!("[COORDINATOR] prepare {}", transaccion.id);

        let id_op = transaccion.id_pago;
        let pago = transaccion.get_pago().unwrap();
        //TODO: Hay que cambiar en el Mensaje el id_op por id_transaccion, sino no vamos a poder reintentar.
        // Preparo los mensajes a enviar
        let m_hotel = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_hotel()}, self.id, id_op);
        let m_aerolinea = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_aerolinea()}, self.id, id_op);
        let m_banco = Mensaje::new(CodigoMensaje::PREPARE { monto: pago.get_monto_hotel() + pago.get_monto_aerolinea()}, self.id, id_op); // TODO: PASAR ESTO AL PARSER 

        // Mensaje esperado
        let esperado = Mensaje::new(CodigoMensaje::READY, self.id, id_op); // TODO: PASAR ESTO AL PARSER 
        
        self.send_and_wait(vec![m_hotel, m_aerolinea, m_banco], esperado)
    }

    fn commit(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log.insertar(transaccion.commit());
        println!("[COORDINATOR] commit {}", transaccion.id);

        let id_op = transaccion.id_pago;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = Mensaje::new(CodigoMensaje::COMMIT, self.id, id_op); // TODO: pensar si hacer que esperado sea finished o no

        self.send_and_wait(vec![mensaje.clone(), mensaje.clone(), mensaje.clone()], mensaje)
    }

    fn abort(&mut self, transaccion: &mut Transaccion) -> Resultado<()> {
        self.log.insertar(transaccion.abort());
        println!("[COORDINATOR] abort {}", transaccion.id);

        let id_op = transaccion.id_pago;

        // Preparo los mensajes a enviar y mensaje esperado
        let mensaje = Mensaje::new(CodigoMensaje::ABORT, self.id, id_op);

        self.send_and_wait(vec![mensaje.clone(), mensaje.clone(), mensaje.clone()], mensaje)
    }

    //Queremos que se encargue de enviarle los mensajes a los destinatarios
    //y espere por sus respuestas, con un timeout y numero de intentos dado
    fn send_and_wait(&mut self, 
                     mensajes: Vec<Mensaje>, 
                     esperado: Mensaje) -> Resultado<()> {

        let mut responses;
        *self.responses.0.lock().unwrap() = vec![None; STAKEHOLDERS];

        loop {
            for (idx, mensaje) in mensajes.iter().enumerate() {
                self.protocolo.enviar(&mensaje, self.destinatarios[idx].clone()).unwrap();
            }
            responses = self.responses.1.wait_timeout_while(self.responses.0.lock().unwrap(), TIMEOUT, |responses| responses.iter().any(Option::is_none));

            if responses.is_ok() { break; }
            println!("[COORDINATOR] timeout {}", esperado.id_op);
        }
        //opt.is_some() && opt.unwrap() == esperado
        if !responses.unwrap().0.iter().all(|opt| opt.as_ref().map_or(false, |r| r == &esperado)) {
            return Err(ErrorApp::Interno(ErrorInterno::new("Respuesta no esperada")));
        }

        Ok(())
    }

    fn responder(mut protocolo: Protocolo, responses: Arc<(Mutex<Vec<Option<Mensaje>>>, Condvar)>) {
        loop {
            let mensaje = protocolo.recibir(None).unwrap(); // TODO: revisar el timeout
            let id_emisor = mensaje.id_emisor;

            match mensaje.codigo {        
                CodigoMensaje::READY => {
                    println!("[COORDINATOR] recibí READY de {}", id_emisor);
                    responses.0.lock().unwrap()[id_emisor] = Some(mensaje);
                    responses.1.notify_all();
                }
                CodigoMensaje::COMMIT => {
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
