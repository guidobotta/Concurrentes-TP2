use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Clase utilizada para la configuracion de variables de entorno.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Envs {
    pub trabajo_min: u64,
    pub trabajo_max: u64,
    pub probabilidad_fallo: f32,
}

impl Envs {
    /// Devuelve un Env con valores por default
    fn new() -> Self {
        Self {
            trabajo_min: 1000,
            trabajo_max: 3000,
            probabilidad_fallo: 0.2,
        }
    }

    /// Lee las variables de entorno de una ruta dada.
    /// Si la ruta no se puede encontrar, valores por defecto son asignados.
    pub fn get_envs<P: AsRef<Path>>(path: P) -> Envs {
        let file = match File::open(path) {
            Ok(r) => r,
            Err(_) => {
                return Envs::new();
            }
        };

        let reader = BufReader::new(file);

        let config: Envs = match serde_json::from_reader(reader) {
            Ok(r) => r,
            Err(_) => Envs::new(),
        };

        config
    }
}
