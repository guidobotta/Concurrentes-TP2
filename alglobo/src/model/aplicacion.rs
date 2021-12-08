use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

use common::error::Resultado;
use super::{leader_election::LeaderElection, escritor_fallidos::EscritorFallidos, log::{Log, Transaccion}};
use super::pago::Pago;
use super::parser::Parser;
use super::coordinador_transaccion::CoordinadorTransaccion;

pub struct Aplicacion {
    handle: JoinHandle<()>,
    continuar: Arc<AtomicBool>
}

impl Aplicacion {
    pub fn new(
        id: usize, 
        lider: LeaderElection, 
        parseador: Parser,
        escritor: EscritorFallidos) -> Resultado<Aplicacion> {
        //let protocolo = Protocolo::new(Aplicacion::direccion_desde_id(id))?;
        let continuar = Arc::new(AtomicBool::new(true));
        let continuar_clonado = continuar.clone();
        Ok(Aplicacion {
            handle: thread::spawn(move || {
                Aplicacion::procesar(id, lider, parseador, escritor, continuar_clonado)
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
        lider: LeaderElection,
        mut parseador: Parser,
        mut escritor: EscritorFallidos,
        continuar: Arc<AtomicBool>,
    ) {
    
        while continuar.load(Ordering::Relaxed) {
            if lider.am_i_leader() {
                Aplicacion::procesar_lider(&lider, &mut parseador, id);
            } else { 
                //No somos el lider, ver que hacer para detectar caida de lider (No hacer busy waiting)
            }
        }
    }

    fn procesar_lider(lider: &LeaderElection, parseador: &mut Parser, id: usize) {
        let mut log = Log::new("./files/estado.log".to_string()).unwrap();
        let mut coordinador = CoordinadorTransaccion::new(id, log.clone());
        let mut parser_fallidos = ParserFallidos::new("./files/fallidos.csv".to_string()).unwrap();
        let mut inicio_lider = true;
        let mut transaccion;
        let mut prox_pago = 0;

        while lider.am_i_leader() {
            //Este if inicio_lider se puede sacar fuera del while, porque ya sabemos que es lider
            if inicio_lider {
                //Aca obtenemos la ultima transaccion que puede o no ser un reintento
                transaccion = log.ultima_transaccion();
                //if transaccion.es_reintento() {
                //    transaccion.pago = parser_fallidos.parsear(transaccion.id_pago).unwrap();
                //    prox_pago = transaccion.id_pago_prox
                //} else {
                //    transaccion.pago = parseador.parsear_nuevo(Some(transaccion.id_pago)).unwrap();
                //}
                //let prox_pago = transaccion.id_pago_prox;
                parseador.actualizar(transaccion.id_pago_prox);
                inicio_lider = false;
            } else if false { // !cola_reintentos.empty? { //Reintento, mensaje por socket.
                //pago = socket.reintento();
                transaccion = log.nueva_transaccion(prox_pago); //Le pasamos prox_pago o que se fije en la ultima transaccion
                //transaccion.id_pago = pago.id;
                transaccion.pago = parseador_fallidos.parsear(transaccion.id_pago);
            } else {
                transaccion = log.nueva_transaccion(prox_pago);
                transaccion.pago = parseador.parsear().unwrap()
            }
            //Procesar transaccion
            if coordinador.submit(&mut transaccion).is_err() {
                //Agregar a la lista de falladas
                println!("El pago de id {} ha fallado", &transaccion.id_pago);
                transaccion.get_pago()
                .and_then(|p| Some(escritor.escribir_fallido(p)));
            }
        }
    }
}