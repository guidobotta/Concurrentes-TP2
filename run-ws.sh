REPLICAS=$(($1 - 1))

if [ $# -eq 0 ]
  then
    echo "Usar como: ./run-ws.sh <num_rep> <-r (opcional para borrar)>"
    exit
fi

if [ $2 = "-r" ]
  then
    rm alglobo/files/fallidos.csv
    rm alglobo/files/estado.log
fi

chmod +x 1-alglobo.sh
chmod +x 2-webservices.sh

for j in `seq 0 2`
do
    echo "Levanto WebService con id $j - $(($j+1))/3"
    gnome-terminal --title="WebService${j}" --geometry "100x20+0+$(($j*360))" -- ./2-webservices.sh $j
done

for i in `seq 0 $REPLICAS`
do
    echo "Levanto Replica con id $i - $(($i+1))/$(($REPLICAS + 1))"
    gnome-terminal --title="AlGlobo${i}" --geometry "100x20+960+$(($i*1080/($REPLICAS+1)))" -- ./1-alglobo.sh $i
done
