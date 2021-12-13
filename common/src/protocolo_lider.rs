use super::error::{ErrorApp, ErrorInterno, Resultado};
use std::net::UdpSocket;
use std::time::Duration;

/// CodigoLider representa el codigo del mensaje lider.
/// # Variantes
/// OK: utilizado para avisar que hay un nodo de mayor prioridad que el que lo
/// recibe.
/// ELECCION: utilizado para llamar a elección.
/// COORDINADOR: utilizado para avisar que hay un nuevo lider.
/// VERIFICAR: utilizado para preguntar si el lider sigue activo.
#[derive(Clone, PartialEq)]
pub enum CodigoLider {
    OK,
    ELECCION,
    COORDINADOR,
    VERIFICAR
}

/// MensajeLider representa un mensaje utilizado para la comunicación en el
/// algoritmo de elección de lider.
#[derive(Clone)]
pub struct MensajeLider {
    pub codigo: CodigoLider,
    pub id_emisor: usize
}

impl MensajeLider {
    /// Devuelve una instancia de MensajeLider.
    /// Recibe el codigo del mensaje y el id del emisor.
    pub fn new(codigo: CodigoLider, id_emisor: usize) -> Self { 
        Self { codigo, id_emisor } 
    }

    /// Convierte el CodigoLider a String y lo devuelve.
    pub fn codificar(&self) -> String {
        match &self.codigo {
            CodigoLider::OK => format!("OK {}", self.id_emisor),
            CodigoLider::ELECCION => format!("ELECCION {}", self.id_emisor),
            CodigoLider::COORDINADOR => format!("COORDINADOR {}", self.id_emisor),
            CodigoLider::VERIFICAR => format!("VERIFICAR {}", self.id_emisor),
        }
    }

    /// Convierte el String a CodigoLider y lo devuelve.
    /// Devuelve error si el String no matchea con algún código.
    pub fn decodificar(mensaje_codificado: &String) -> Resultado<MensajeLider> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        let codigo = match parseado[0] {
            "OK" => CodigoLider::OK,
            "ELECCION" => CodigoLider::ELECCION,
            "COORDINADOR" => CodigoLider::COORDINADOR,
            "VERIFICAR" => CodigoLider::VERIFICAR,
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        };

        Ok(MensajeLider::new(
            codigo, 
            parseado[1].parse::<usize>()?, 
        ))
    }
}

impl PartialEq for MensajeLider {
    // TODO: documentar?? Es privada
    fn eq(&self, otro: &Self) -> bool {
        self.codigo == otro.codigo
    }
}

static TAM_BUFFER: usize = 256;

/// ProtocoloLider encapsula la comunicación socket de parte del algoritmo de
/// elección de lider. Implementa el envío y la recepción de mensajes
/// encapsulando la codificación y decodificación de estos.
pub struct ProtocoloLider {
    skt: UdpSocket,
}

impl ProtocoloLider {
    /// Devuelve una instancia de ProtocoloLider.
    /// Recibe la direccion a la que se va a bindear el socket.
    pub fn new(direccion: String) -> Resultado<ProtocoloLider> {
        Ok(ProtocoloLider {
            skt: UdpSocket::bind(direccion)?,
        })
    }

    /// Recibe un mensaje y una direccion. Codifica el mensaje y lo envía a
    /// dicha dirección.
    pub fn enviar(&mut self, mensaje: &MensajeLider, direccion: String) -> Resultado<()> {
        let mensaje = mensaje.codificar();
        self.skt.send_to(mensaje.as_bytes(), direccion)?;
        Ok(())
    }

    /// Recibe un timeout. Si el timeout en None, se bloquea hasta recibir un
    /// mensaje. Sino, devuelve error si hay ocurre timeout.
    pub fn recibir(&mut self, timeout: Option<Duration>) -> Resultado<MensajeLider> {
        let mut buffer = vec![0; TAM_BUFFER];
        if self.skt.set_read_timeout(timeout).is_err() {
            return Err(ErrorApp::Interno(ErrorInterno::new("Error al setear timeout")));
        };
        let (recibido, _src) = self.skt.recv_from(&mut buffer)?;

        if recibido == 0 {
            return Err(ErrorApp::Interno(ErrorInterno::new("Timeout en recepcion")));
        }
        MensajeLider::decodificar(&String::from_utf8(buffer[..recibido].to_vec())?)
    }

    /// Devuelve una copia de ProtocoloLider
    pub fn clone(&self) -> Self {
        ProtocoloLider {
            skt: self.skt.try_clone().expect("Error al intentar clonar el socket en ProtocoloLider"),
        }
    }
}
