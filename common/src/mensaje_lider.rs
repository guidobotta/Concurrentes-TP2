use super::error::{ErrorApp, ErrorInterno, Resultado};

#[derive(Clone, PartialEq)]
pub enum CodigoLider {
    OK,
    ELECCION,
    COORDINADOR
}

#[derive(Clone)]
pub struct MensajeLider {
    pub codigo: CodigoLider,
    pub id_emisor: usize
}

impl MensajeLider {
    pub fn new(codigo: CodigoLider, id_emisor: usize) -> Self { 
        Self { codigo, id_emisor } 
    }

    pub fn codificar(&self) -> String {
        match &self.codigo {
            CodigoLider::OK => format!("OK {}", self.id_emisor),
            CodigoLider::ELECCION => format!("ELECCION {}", self.id_emisor),
            CodigoLider::COORDINADOR => format!("COORDINADOR {}", self.id_emisor),
        }
    }

    pub fn decodificar(mensaje_codificado: &String) -> Resultado<MensajeLider> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        let codigo = match parseado[0] {
            "OK" => CodigoLider::OK,
            "ELECCION" => CodigoLider::ELECCION,
            "COORDINADOR" => CodigoLider::COORDINADOR,
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        };

        Ok(MensajeLider::new(
            codigo, 
            parseado[1].parse::<usize>()?, 
        ))
    }
}

impl PartialEq for MensajeLider {
    fn eq(&self, otro: &Self) -> bool {
        self.codigo == otro.codigo
    }
}