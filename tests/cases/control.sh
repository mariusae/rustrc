rc -c 'if(~ 1 1) echo yes; if not echo no; if(~ 1 2) echo yes; if not echo no'
rc -c 'for(i in 1 2 3) echo i=$i; for(i in) echo empty; for(i in ()) echo empty2; echo end'
rc -c 'for(i) echo arg=$i' a b c
rc -c 'i=0; while(! ~ $i 3) { echo w$i; i=`{expr $i + 1} }'
rc -c 'while() { echo once; exit 5 }'; echo "code=$?"
rc -c 'switch(b){
case a
	echo A
case b c
	echo BC
case *
	echo default
}
switch(zzz){
case a
	echo A
case *
	echo default
}
switch(x){
case a
}
echo after'
rc -c 'switch(a b) {
case *
	echo star $status
}'
rc -c 'x=abc; switch($x) { case a* ; echo starts-with-a }'
rc -c 'switch(a){case a; echo a1; case a; echo a2}'
rc -c 'true && echo and1; false && echo and2; true || echo or1; false || echo or2; echo $status'
rc -c 'false && echo x || echo y; true || echo x && echo z'
rc -c '! true | false; echo $status'
rc -c '@{cd /; pwd}; pwd' | sed "s|$WORK|WORK|"
rc -c '@ x=1; echo -$x-'
rc -c 'x=1 { echo $x }; echo -$x-'
rc -c 'x=1 y=2 echo $x $y; echo -$x$y-'
rc -c 'x=1 y=2; echo $x $y'
rc -c 'x=1; x=2 true; echo $x; x=3 echo $x; echo $x'
rc -c 'if(x=5 true) echo $x'
rc -c '{echo a; echo b} | wc -l'
rc -c '{echo a; echo b} >/dev/null; echo c'
rc -c 'if (false) echo a
if not {
	echo b
}'
rc -c 'for(i in a b) if(~ $i b) echo found $i'
rc -c 'if(true) if(false) echo x; if not echo y'
