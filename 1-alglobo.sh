RUTA="./files/1.csv"
FALLIDOS="./files/fallidos.csv"
ID=$1

cd alglobo/
cargo run $RUTA $FALLIDOS $ID