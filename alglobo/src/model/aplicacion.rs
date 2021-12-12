use std::sync::{,
    Arc, RwLock,
};
use std::thread::{self, JoinHandle};

use common::error::Resultado;
use super::{eleccion_lider::EleccionLider, parser_fallidos::ParserFallidos, log::{Log, Transaccion}, comando::Comando};
use super::parser::Parser;
use super::coordinador_transaccion::CoordinadorTransaccion;
use std::sync::mpsc::Receiver;

pub struct Aplicacion {
    handle: JoinHandle<()>
}

pub enum EstadoApp {
    FinEntrada,
    CambioLider,
    Finalizar
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

        let mut estado = EstadoApp::CambioLider;

        while lider.bloquear_si_no_soy_lider() {
            //TODO: Ver que hacer con los errores
            match estado {
                EstadoApp::CambioLider => {
                    match Aplicacion::procesar_lider(&lider, &mut parseador, &mut receptor, id) {
                        Ok(r) => {estado = r},
                        Err(e) => println!("{}", e)
                    }
                },
                EstadoApp::FinEntrada => {
                    match Aplicacion::procesar_fallidos(&lider, &mut receptor, id) {
                        Ok(r) => {estado = r},
                        Err(e) => println!("{}", e)
                    }
                },
                EstadoApp::Finalizar => {
                    println!("[Aplicacion]: Finalizando...");
                    lider.finalizar();
                    break;
                }
            }
        }
    }

    fn procesar_lider(
        lider: &EleccionLider, 
        parseador: &mut Parser, 
        receptor: &mut Receiver<Comando>, 
        id: usize) -> Resultado<EstadoApp> {

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
                    Comando::FINALIZAR => return Ok(EstadoApp::Finalizar),
                    Comando::REINTENTAR {id} => id
                };
                transaccion = match Aplicacion::procesar_comando(id_reintento, &mut parser_fallidos, &mut log, prox_pago) {
                    Ok(Some(t)) => t,
                    _ => continue 
                };
            } else {
                transaccion = log.read().unwrap().nueva_transaccion(prox_pago);
                transaccion.pago = match parseador.parsear_nuevo(Some(prox_pago)).ok() {
                    Some(None) => return Ok(EstadoApp::FinEntrada),
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

        Ok(EstadoApp::CambioLider)
    }


    fn procesar_comando(id_reintento: usize, 
                        parser: &mut ParserFallidos,
                        log: &mut Arc<RwLock<Log>>,
                        prox_pago: usize) -> Resultado<Option<Transaccion>>{
                            
        let mut transaccion = log.read().unwrap().nueva_transaccion(prox_pago); //Le pasamos prox_pago o que se fije en la ultima transaccion
        transaccion.id_pago = id_reintento;
        transaccion.id_pago_prox = prox_pago;

        match parser.parsear_fallido(id_reintento) {
            Ok(Some(pago)) => {
                println!("[Aplicacion]: Se reintenta el pago de id {}", id_reintento);
                transaccion.pago = Some(pago)
            },
            Ok(None) => {
                println!("[Aplicacion]: Error, no se encontró el pago de id {} en el archivo de fallidos", id_reintento);
                return Ok(None)
            },
            Err(e) => {
                println!("{}", e);
                return Err(e)
            }
        };

        Ok(Some(transaccion))
    }

    fn procesar_fallidos(
        lider: &EleccionLider,
        receptor: &mut Receiver<Comando>, 
        id: usize) -> Resultado<EstadoApp> {

        let log = Arc::new(RwLock::new(Log::new("./files/estado.log".to_string()).unwrap()));
        let mut coordinador = CoordinadorTransaccion::new(id, log.clone());
        let mut parser_fallidos = ParserFallidos::new("./files/fallidos.csv".to_string()).unwrap();
        let mut transaccion;
        let prox_pago = log.read().unwrap()
                                            .ultima_transaccion()
                                            .and_then(|t| Some(t.id_pago_prox))
                                            .unwrap_or(1);
        
        while lider.soy_lider() {
            if let Ok(comando) = receptor.recv() {
                let id_reintento = match comando {
                    Comando::FINALIZAR => return Ok(EstadoApp::Finalizar),
                    Comando::REINTENTAR {id} => id
                };
                transaccion = match Aplicacion::procesar_comando(id_reintento, &mut parser_fallidos, &mut log, prox_pago) {
                    Ok(Some(t)) => t,
                    _ => continue 
                };
                if coordinador.submit(&mut transaccion).is_err() {
                    //Agregar a la lista de falladas
                    println!("[APLICACION]: El pago de id {} ha fallado", &transaccion.id_pago);
                    transaccion.get_pago()
                    .and_then(|p| Some(parser_fallidos.escribir_fallido(p)));
                }
            }
        }

        Ok(EstadoApp::CambioLider)
    }

    pub fn join(self) {
        let _ = self.handle.join();
    }
}