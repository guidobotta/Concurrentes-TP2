use common::error::{ErrorApp, ErrorInterno, Resultado};

/// Comando enumera los posibles comandos.
/// # Variantes
/// REINTENTAR: simboliza un intento y contiene el id del pago correspondiente
/// FINALIZAR: simboliza la finalización de la ejecución de la aplicación
#[derive(Clone, PartialEq, Debug)]
pub enum Comando {
    REINTENTAR { id: usize },
    FINALIZAR,
}

impl Comando {
    /// Recibe una cadena y devuelve la cadena decodificada.
    /// Devuelve error si la cadena no corresponde a ninguna de las variantes.
    pub fn decodificar(mensaje_codificado: &String) -> Resultado<Comando> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        match parseado[0] {
            "REINTENTAR" => Ok(Comando::REINTENTAR {
                id: parseado[1].parse::<usize>()?,
            }),
            "FINALIZAR" => Ok(Comando::FINALIZAR),
            _ => {
                return Err(ErrorApp::Interno(ErrorInterno::new(&format!(
                    "Mensaje erroneo: {}",
                    parseado[0]
                ))))
            }
        }
    }
}
