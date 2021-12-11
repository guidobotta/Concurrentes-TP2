use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::thread::{self, JoinHandle};

use common::error::Resultado;
use super::{eleccion_lider::EleccionLider, parser_fallidos::ParserFallidos, log::Log, comando::Comando};
use super::parser::Parser;
use super::coordinador_transaccion::CoordinadorTransaccion;
use std::sync::mpsc::Receiver;

pub struct Aplicacion {
    handle: JoinHandle<()>
}

impl Aplicacion {
    pub fn new(
        id: usize, 
        lider: EleccionLider, 
        parseador: Parser,
        receptor: Receiver<Comando>) -> Resultado<Aplicacion> {
        Ok(Aplicacion {
            handle: thread::spawn(move || {
                Aplicacion::procesar(id, lider, parseador, receptor)
            })
        })
    }

    fn procesar(
        id: usize,
        mut lider: EleccionLider,
        mut parseador: Parser,
        mut receptor: Receiver<Comando>
    ) {
        while lider.bloquear_si_no_soy_lider() {
            //TODO: Ver que hacer con los errores
            if let Ok(None) = Aplicacion::procesar_lider(&lider, &mut parseador, &mut receptor, id) { 
                println!("Antes de finalizar");
                lider.finalizar();
                break; 
            }
        }

        println!("Fin en procesar");
    }

    fn procesar_lider(
        lider: &EleccionLider, 
        parseador: &mut Parser, 
        receptor: &mut Receiver<Comando>, 
        id: usize) -> Resultado<Option<()>> {

        let log = Arc::new(RwLock::new(Log::new("./files/estado.log".to_string()).unwrap()));
        let mut coordinador = CoordinadorTransaccion::new(id, log.clone());
        let mut parser_fallidos = ParserFallidos::new("./files/fallidos.csv".to_string()).unwrap();
        let mut inicio_lider = true;
        let mut transaccion;
        let mut prox_pago = 1;


        while lider.soy_lider() {
            //Este if inicio_lider se puede sacar fuera del while, porque ya sabemos que es lider
            if inicio_lider {
                inicio_lider = false;
                transaccion = match log.read().unwrap().ultima_transaccion() {
                    Some(t) => t,
                    None => continue
                };
                prox_pago = transaccion.id_pago_prox;
                transaccion.pago = match parseador.parsear_nuevo(Some(transaccion.id_pago)).ok() {
                    Some(Some(p)) => Some(p),
                    _ => { panic!("ERROR: El log de transacciones no matchea con el archivo de entrada") }
                };
            } else if let Ok(comando) = receptor.try_recv() {
                let id_reintento = match comando {
                    Comando::FINALIZAR => return Ok(None),
                    Comando::REINTENTAR {id} => id
                };
                println!("[Aplicacion]: Se reintenta el pago de id {}", id_reintento);
                transaccion = log.read().unwrap().nueva_transaccion(prox_pago); //Le pasamos prox_pago o que se fije en la ultima transaccion
                transaccion.id_pago = id_reintento;
                transaccion.id_pago_prox = prox_pago;
                transaccion.pago = Some(parser_fallidos.parsear_fallido(id_reintento).unwrap().unwrap());
            } else {
                transaccion = log.read().unwrap().nueva_transaccion(prox_pago);
                transaccion.pago = match parseador.parsear_nuevo(Some(prox_pago)).ok() {
                    Some(None) => return Ok(None),
                    Some(p) => p,
                    _ => {panic!("Algo malo paso")}
                };
                prox_pago += 1;
            }
            //Procesar transaccion
            if coordinador.submit(&mut transaccion).is_err() {
                //Agregar a la lista de falladas
                println!("[APLICACION]: El pago de id {} ha fallado", &transaccion.id_pago);
                transaccion.get_pago()
                .and_then(|p| Some(parser_fallidos.escribir_fallido(p)));
            }
        }

        //println!("Fin en procesar_lider");
        //coordinador.finalizar();
        Ok(Some(()))
    }


    pub fn join(self) {
        let _ = self.handle.join();
    }
}