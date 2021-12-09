REPLICAS=$(($1 - 1))

if [ $# -eq 0 ]
  then
    echo "Usar como: ./run-ws.sh <num_rep>"
    exit
fi

rm alglobo/files/fallidos.csv
rm alglobo/files/estado.log

chmod +x 1-alglobo.sh
chmod +x 2-webservices.sh

gnome-terminal -- ./2-webservices.sh 0
gnome-terminal -- ./2-webservices.sh 1
gnome-terminal -- ./2-webservices.sh 2

for i in `seq 0 $REPLICAS`
do
    echo "Levanto Replica con id $i - $(($i+1))/$(($REPLICAS + 1))"
    gnome-terminal -- ./1-alglobo.sh ./files/1.csv ./files/fallidos.csv $i
done
