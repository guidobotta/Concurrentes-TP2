use super::error::{ErrorApp, ErrorInterno, Resultado};
use std::net::UdpSocket;
use std::time::Duration;

static TAM_BUFFER: usize = 256;

/// CodigoTransaccion representa el codigo del mensaje de transacción.
/// # Variantes
/// PREPARE: utilizado para avisar que tomen recursos.
/// READY: utilizado para avisar que se tomaron recursos y están listos para
/// hacer el commit.
/// COMMIT: utilizado tanto para avisar que se haga el commit como para avisar
/// que se terminó el commit.
/// ABORT: utilizado tanto para avisar que se haga el abort como para avisar
/// que se terminó el abort.
#[derive(Clone, PartialEq, Debug)]
pub enum CodigoTransaccion {
    PREPARE { monto: f64 },
    READY,
    COMMIT,
    ABORT
}

/// MensajeTransaccion representa un mensaje utilizado para la comunicación en
/// el algoritmo de transaccionalidad.
#[derive(Clone)]
pub struct MensajeTransaccion {
    pub codigo: CodigoTransaccion,
    pub id_emisor: usize,
    pub id_op: usize
}

impl MensajeTransaccion {
    /// Devuelve una instancia de MensajeTransaccion.
    /// Recibe el codigo del mensaje, el id del emisor y el id de la operacion.
    pub fn new(codigo: CodigoTransaccion, id_emisor: usize, id_op: usize) -> Self { 
        Self { codigo, id_emisor, id_op } 
    }

    /// Convierte el CodigoTransaccion a String y lo devuelve.
    pub fn codificar(&self) -> String {
        match &self.codigo {
            CodigoTransaccion::PREPARE { monto } => format!("PREPARE {} {} {}", self.id_emisor, self.id_op, monto),
            CodigoTransaccion::COMMIT => format!("COMMIT {} {}", self.id_emisor, self.id_op),
            CodigoTransaccion::READY => format!("READY {} {}", self.id_emisor, self.id_op),
            CodigoTransaccion::ABORT => format!("ABORT {} {}", self.id_emisor, self.id_op),
        }
    }

    /// Convierte el String a CodigoTransaccion y lo devuelve.
    /// Devuelve error si el String no matchea con algún código.
    pub fn decodificar(mensaje_codificado: &String) -> Resultado<MensajeTransaccion> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        let codigo = match parseado[0] {
            "PREPARE" => CodigoTransaccion::PREPARE { monto: parseado[3].parse::<f64>()? },
            "COMMIT" => CodigoTransaccion::COMMIT,
            "ABORT" => CodigoTransaccion::ABORT,
            "READY" => CodigoTransaccion::READY,
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        };

        Ok(MensajeTransaccion::new(
            codigo, 
            parseado[1].parse::<usize>()?, 
            parseado[2].parse::<usize>()?
        ))
    }
}

impl PartialEq for MensajeTransaccion {
    /// Devuelve verdadero si el codigo y el id_op coinciden
    fn eq(&self, otro: &Self) -> bool {
        self.codigo == otro.codigo && self.id_op == self.id_op
    }
}

/// ProtocoloTransaccion encapsula la comunicación socket de parte del algoritmo
/// de transaccionalidad. Implementa el envío y la recepción de mensajes
/// encapsulando la codificación y decodificación de estos.
pub struct ProtocoloTransaccion {
    skt: UdpSocket,
}

impl ProtocoloTransaccion {
    /// Devuelve una instancia de ProtocoloTransaccion.
    /// Recibe la direccion a la que se va a bindear el socket.
    pub fn new(direccion: String) -> Resultado<ProtocoloTransaccion> {
        Ok(ProtocoloTransaccion {
            skt: UdpSocket::bind(direccion)?,
        })
    }

    /// Recibe un mensaje y una direccion. Codifica el mensaje y lo envía a
    /// dicha dirección.
    pub fn enviar(&mut self, mensaje: &MensajeTransaccion, direccion: String) -> Resultado<()> {
        let mensaje = mensaje.codificar();
        self.skt.send_to(mensaje.as_bytes(), direccion)?;
        Ok(())
    }

    /// Recibe un timeout. Si el timeout en None, se bloquea hasta recibir un
    /// mensaje. Sino, devuelve error si hay ocurre timeout.
    pub fn recibir(&mut self, timeout: Option<Duration>) -> Resultado<MensajeTransaccion> {
        let mut buffer = vec![0; TAM_BUFFER];
        if self.skt.set_read_timeout(timeout).is_err() {
            return Err(ErrorApp::Interno(ErrorInterno::new("Error al setear timeout")));
        };
        let (recibido, _src) = self.skt.recv_from(&mut buffer)?;

        if recibido == 0 {
            return Err(ErrorApp::Interno(ErrorInterno::new("Timeout en recepcion")));
        }
        MensajeTransaccion::decodificar(&String::from_utf8(buffer[..recibido].to_vec())?)
    }

    /// Devuelve una copia de ProtocoloTransaccion
    pub fn clone(&self) -> Resultado<Self> {
        Ok(ProtocoloTransaccion {
            skt: self.skt.try_clone()?,
        })
    }
}
