ID=$1

if [ $# -eq 0 ]
  then
    echo "Usar como: ./nodo-alglobo.sh <id>"
    exit
fi

gnome-terminal --title="AlGlobo$ID" --geometry "100x20+0+0" -- ./1-alglobo.sh $ID
