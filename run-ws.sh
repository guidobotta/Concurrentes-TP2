chmod +x 1-alglobo.sh
chmod +x 2-webservices.sh

gnome-terminal -- ./2-webservices.sh 0
gnome-terminal -- ./2-webservices.sh 1
gnome-terminal -- ./2-webservices.sh 2

gnome-terminal -- ./1-alglobo.sh ./files/1.csv ./files/fallidos.csv 3
gnome-terminal -- ./1-alglobo.sh ./files/1.csv ./files/fallidos.csv 4
