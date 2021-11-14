use super::leader_election::LeaderElection;


pub struct Aplicacion {
    id: usize,
    lider: LeaderElection
}

impl Aplicacion {

    fn new(id: usize, lider: LeaderElection) -> Aplicacion {
        Aplicacion {
            id,
            lider
        }
    }

    fn comenzar(&mut self) {
        //Spawn del thread
    }

    fn finalizar(self) {
        //Join del thread
    }

    
}