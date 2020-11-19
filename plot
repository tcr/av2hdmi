#!/bin/bash

gnuplot -e "set term png size 50000,500 font 'sans,8'; \
 set title '41.7 Msample/s'; set grid; set key noautotitle; \
 set output 'test2.png'; set samples 50000; \
 plot 20*sin((x+4.8)/(41.66/(3.58*2*pi))) , \
 'out2.csv' every ::1 with lines; "
