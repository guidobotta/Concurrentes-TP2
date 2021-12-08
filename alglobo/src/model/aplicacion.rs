use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

use common::error::Resultado;
use super::{eleccion_lider::EleccionLider, parser_fallidos::ParserFallidos, log::Log};
use super::parser::Parser;
use super::coordinador_transaccion::CoordinadorTransaccion;

pub struct Aplicacion {
    handle: JoinHandle<()>,
    continuar: Arc<AtomicBool>
}

impl Aplicacion {
    pub fn new(
        id: usize, 
        lider: EleccionLider, 
        parseador: Parser) -> Resultado<Aplicacion> {
        //let protocolo = Protocolo::new(Aplicacion::direccion_desde_id(id))?;
        let continuar = Arc::new(AtomicBool::new(true));
        let continuar_clonado = continuar.clone();
        Ok(Aplicacion {
            handle: thread::spawn(move || {
                Aplicacion::procesar(id, lider, parseador, continuar_clonado)
            }),
            continuar,
        })
    }

    pub fn finalizar(self) {
        self.continuar.store(false, Ordering::Relaxed); //Ver si el Ordering Relaxed esta bien
        let _ = self.handle.join();
    }

    fn procesar(
        id: usize,
        lider: EleccionLider,
        mut parseador: Parser,
        continuar: Arc<AtomicBool>,
    ) {
    
        while continuar.load(Ordering::Relaxed) {
            if lider.soy_lider() {
                Aplicacion::procesar_lider(&lider, &mut parseador, id);
            } else { 
                //No somos el lider, ver que hacer para detectar caida de lider (No hacer busy waiting)
            }
        }
    }

    fn procesar_lider(lider: &EleccionLider, parseador: &mut Parser, id: usize) {
        let mut log = Log::new("./files/estado.log".to_string()).unwrap();
        let mut coordinador = CoordinadorTransaccion::new(id, log.clone());
        let mut parser_fallidos = ParserFallidos::new("./files/fallidos.csv".to_string()).unwrap();
        let mut inicio_lider = true;
        let mut transaccion;
        let mut prox_pago = 1;

        while lider.soy_lider() {
            //Este if inicio_lider se puede sacar fuera del while, porque ya sabemos que es lider
            if inicio_lider {
                inicio_lider = false;
                transaccion = match log.ultima_transaccion() {
                    Some(t) => t,
                    None => continue
                };
                prox_pago = transaccion.id_pago_prox;
            } else if false { // !cola_reintentos.empty? { //Reintento, mensaje por socket.
                //pago = socket.reintento();
                transaccion = log.nueva_transaccion(prox_pago); //Le pasamos prox_pago o que se fije en la ultima transaccion
                //transaccion.id_pago = pago.id;
                transaccion.pago = Some(parser_fallidos.parsear_fallido(transaccion.id_pago).unwrap().unwrap());
            } else {
                transaccion = log.nueva_transaccion(prox_pago);
                transaccion.pago = match parseador.parsear_nuevo(None).ok() {
                    Some(None) => break,
                    Some(p) => p,
                    _ => {panic!("Algo malo paso")}
                };
            }
            //Procesar transaccion
            if coordinador.submit(&mut transaccion).is_err() {
                //Agregar a la lista de falladas
                println!("El pago de id {} ha fallado", &transaccion.id_pago);
                transaccion.get_pago()
                .and_then(|p| Some(parser_fallidos.escribir_fallido(p)));
            }
        }
    }
}