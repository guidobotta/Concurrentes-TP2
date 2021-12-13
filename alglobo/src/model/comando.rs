use common::error::{ErrorApp, ErrorInterno, Resultado};

/// Comando enumera los posibles comandos.
/// # Variantes
/// Reintentar: simboliza un intento y contiene el id del pago correspondiente
/// Finalizar: simboliza la finalización de la ejecución de la aplicación
#[derive(Clone, PartialEq, Debug)]
pub enum Comando {
    Reintentar { id: usize },
    Finalizar,
}

impl Comando {
    /// Recibe una cadena y devuelve la cadena decodificada.
    /// Devuelve error si la cadena no corresponde a ninguna de las variantes.
    pub fn decodificar(mensaje_codificado: &str) -> Resultado<Comando> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        match parseado[0] {
            "R" => Ok(Comando::Reintentar {
                id: parseado[1].parse::<usize>()?,
            }),
            "F" => Ok(Comando::Finalizar),
            _ => {
                return Err(ErrorApp::Interno(ErrorInterno::new(&format!(
                    "Mensaje erroneo: {}",
                    parseado[0]
                ))))
            }
        }
    }
}
