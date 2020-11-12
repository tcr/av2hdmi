gnuplot -e "set term png size 12800,500 font 'sans,8'; \
 set title '41.7 Msample/s'; set grid; set key noautotitle; \
 set output 'test6.png'; plot 'out.csv' every ::10 with lines"

./fb2d


https://iosoft.blog/2020/05/25/raspberry-pi-dma-programming/
