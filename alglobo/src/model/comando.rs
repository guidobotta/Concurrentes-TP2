use common::error::{ErrorApp, ErrorInterno, Resultado};

#[derive(Clone, PartialEq, Debug)]
pub enum Comando {
    REINTENTAR { id: usize },
    FINALIZAR
}

//#[derive(Clone)]
//pub struct Comando {
//    pub codigo: CodigoMensaje
//}

//use std::any::type_name;
//
//fn type_of<T>(_: T) -> &'static str {
//    type_name::<T>()
//}

impl Comando {
    //pub fn new(codigo: CodigoComando, id_emisor: usize, id_op: usize) -> Self { 
    //    Self { codigo, id_emisor, id_op } 
    //}

    pub fn decodificar(mensaje_codificado: &String) -> Resultado<Comando> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        println!("PARSEADO: {:?}, {:?}", &parseado.get(0), parseado.get(1));
        match parseado[0] {
            "REINTENTAR" => Ok(Comando::REINTENTAR { id: parseado[1].parse::<usize>()? }),
            "FINALIZAR" => Ok(Comando::FINALIZAR),
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        }
    }
}