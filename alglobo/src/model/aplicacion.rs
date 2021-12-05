use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

use common::error::Resultado;
use super::{leader_election::LeaderElection, escritor_fallidos::EscritorFallidos};
use super::pago::Pago;
use super::parser::Parser;
use super::coordinador_transaccion::CoordinadorTransaccion;

static NUMERO_REPLICAS: usize = 10;
static TIMEOUT: usize = 3000; //Milis

pub struct Aplicacion {
    handle: JoinHandle<()>,
    continuar: Arc<AtomicBool>,
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
        let mut coordinador = CoordinadorTransaccion::new(id);
        while continuar.load(Ordering::Relaxed) {
            if lider.am_i_leader() {
                //if !conexion_establecida {
                //    //Conectar con webservices
                //    Aplicacion::sincronizar();
                //}

                let pago = match parseador.parsear_pago().ok() {
                    Some(Some(r)) => r,
                    _ => break,
                };

                //Procesar pago
                let id_pago = pago.get_id();
                if coordinador.submit(pago.clone()).is_err() {
                    //Agregar a la lista de falladas
                    println!("El pago de id {} ha fallado", id_pago);
                    escritor.escribir_fallido(pago);
                }

                //Aplicacion::actualizar_replicas(&mut protocolo, parseador.posicion());
            } else {
                //Recibir actualizacion del archivo (TIMEOUT)
                //Si la actualizacion es nueva (la linea es mayor a la que tengo en mi estado):
                //Actualizar mi estado
                //Replicar el mensaje a todas las replicas (ver si filtrar el lider)
                //Solicitar nuevo lider si salta timeout
            }

            //Si es lider:
            //Si conexion no establecida:
            //Conectar con webservices.
            //Sincronizar con las replicas:
            //El nuevo lider envia varios mensajes (en rafaga o en tandas) de "Obtener estado" a todas las replicas
            //Se queda con el estado (la linea) mas alta de todas
            //Leer una linea del archivo
            //Procesar pago (enviar a los webservices)
            //Si falla agregar a lista de falladas
            //Actualizar replicas
            //1. Enviar a cada replica por UDP un mensaje de "Estoy parado en esta linea"
            //2. Quedarse esperando por la respuesta de almenos uno

            //Si no es el lider:
            //Recibir actualizacion del archivo (TIMEOUT)
            //Si la actualizacion es nueva (la linea es mayor a la que tengo en mi estado):
            //Actualizar mi estado
            //Replicar el mensaje a todas las replicas (ver si filtrar el lider)
            //Solicitar nuevo lider si salta timeout
        }
    }

    fn sincronizar() {
        //Enviar varios mensajes (en rafaga o en tandas) de "Obtener estado" a todas las replicas
        //Quedarse con el estado (la linea) mas alta de todas
    }

    fn procesar_pago(_pago: &Pago) -> Resultado<()> {
        //Procesar transaccionalidad a los webservices
        
        Ok(())
    }
}


//Nuestra aplicacion levanta un archivo de pagos y lo lee secuencialmente.
//Para reintentar los pagos no finalizados, podemos almacenarlos en un archivo y reintentarlos secuencialmente?
//Instanciamos la aplicacion con el nuevo path