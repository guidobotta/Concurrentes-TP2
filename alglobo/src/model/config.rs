pub struct Config {}

impl Config {
    pub fn ruta_fallidos() -> String {
        "./files/fallidos.csv".to_string()
    }

    pub fn ruta_logs() -> String {
        "./files/estado.log".to_string()
    }
}
