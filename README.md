# TP2-Concurrentes
Trabajo Práctico 2 - Técnicas de Programación Concurrente - Grupo Matambre a la Pizza

## Estructura de Directorios

```
├── alglobo
├── common
└── webservices
```

- Dentro de alglobo/ se encuentra el código fuente de la aplicación de AlGlobo.
- Dentro de webservice/ se encuentra el código fuente de la aplicación de WebService.
- Dentro de common/ se encuentra el código fuente de ciertas estructuras utilizadas en ambas aplicaciones.

## Scripts 

Se generaron distintos scripts en shell scripting y python.

```
├── 1-alglobo.sh
├── 2-webservices.sh
├── ex_gen.py
├── nodo-alglobo.sh
├── nodo-webservice.sh
└── run-ws.sh
```

- `./1-alglobo.sh <ID>` levanta un nodo de alglobo en la misma terminal en la que se lo ejecuta con el ID indicado por parámetro con el archivo `./files/1.csv` como archivo de entrada (se debe cambiar desde el script para cambiar el archivo)
- `./2-webservices.sh <ID>` levanta un webservice con el ID pasado por parámetro.
  - ID 0 es la aerolinea
  - ID 1 es el hotel
  - ID 2 es el banco
- `./nodo-alglobo.sh <ID>` hace lo mismo que `1-alglobo.sh` pero crea una nueva terminal y lo ejecuta en esa terminal.
- `./nodo-webservice.sh <ID>` hace lo mismo que `2-webservices.sh` pero crea una nueva terminal y lo ejecuta en esa terminal.
- `./run-ws.sh <CANT_NODOS> <-r (opcional)>` recibe la cantidad de nodos de AlGlobo a levantar y un parámetro opcional `-r` para eliminar los archivos `fallidos.csv` y `estado.log`. Levanta `<CANT_NODOS>` nodos de AlGlobo y los 3 WebServices en distintas terminales.
- `python3 ex_gen.py <CANT_PAGOS>` recibe la cantidad de pagos a crear y crea un archivo csv con dicha cantidad de entradas en `./alglobo/files/example-{<CANT_PAGOS>}.csv`.
