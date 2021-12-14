ID=$1

if [ $# -eq 0 ]
  then
    echo "Usar como: ./nodo-webservice.sh <id>"
    exit
fi

gnome-terminal --title="WebService$ID" --geometry "100x20+0+0" -- ./2-webservices.sh $ID
