use std::thread::{self, JoinHandle};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use super::leader_election::LeaderElection;
use super::parser::Parser;


pub struct Aplicacion {
    handle: JoinHandle<()>,
    continuar: Arc<AtomicBool>
}

impl Aplicacion {

    pub fn new(id: usize, lider: LeaderElection, parseador: Parser) -> Aplicacion {
        let continuar = Arc::new(AtomicBool::new(true));
        let continuar_clonado = continuar.clone();
        Aplicacion {
            handle: thread::spawn(move || Aplicacion::procesar(id, lider, parseador, continuar_clonado)),
            continuar
        }
    }

    pub fn finalizar(self) {
        self.continuar.store(false, Ordering::Relaxed); //Ver si el Ordering Relaxed esta bien
        let _ = self.handle.join();
    }

    fn procesar(id: usize, lider: LeaderElection, parseador: Parser, continuar: Arc<AtomicBool>) {
        while continuar.load(Ordering::Relaxed) {
            //Si es lider:
                //Si conexion no establecida:
                    //Conectar con webservices.
                    //Sincronizar con las replicas:
                        //El nuevo lider envia varios mensajes (en rafaga o en tandas) de "Obtener estado" a todas las replicas
                        //Se queda con el estado (la linea) mas alta de todas
                        //Almacena los streams (sockets) de los nodos que respondieron
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
}