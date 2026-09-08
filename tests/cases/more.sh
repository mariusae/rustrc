rc -c 'fn f {fn g {echo inner $1}; g $1}; f x; g y; whatis g'
rc -c 'fn f {fn f {echo second}; echo first}; f; f'
rc -c 'fn f {echo $1; fn f}; f a; f b; echo st=$status'
rc -c 'fn f {. src.rc $*; echo back $#*}; f 1 2 3; echo $x'
rc -c 'fn f {shift; . src.rc $*}; f 1 2; echo $#*' a b
rc -c 'for(i in *.c c.h) echo $i; for(i in *) echo $i' 
rc -c 'switch(a.c){case *.c; echo c-file; case *; echo other}; switch(*.c){case *; echo star}; x=(*.c); switch($x){case a.c; echo a; case *; echo no}'
rc -c 'switch(a b){case a; echo a; case *; echo other}; switch((a b)){case '"'"'a b'"'"'; echo ab}; switch($*){case '"'"'1 2'"'"'; echo one-two}' 1 2
rc -c 'switch(x){case a b x; echo hit}; switch(x){case (a b); echo list}; switch(x){case; echo empty; case *; echo star}'
rc -c 'switch(x){case `{echo x}; echo bq}; y=x; switch(x){case $y; echo var; case *; echo star}'
rc -c 'while(echo cond | grep -q c) {x=($x 1); if(~ $#x 3) exit 0}; echo notreached'; echo "code=$?"
rc -c 'while(~ $#x 0 1) {x=($x a); echo loop $#x}; echo done $#x'
rc -c 'i=0; while(test $i -lt 3) {echo $i; i=`{expr $i + 1}}; {i=0; while(test $i -lt 2) {echo p$i; i=`{expr $i + 1}}} | cat'
rc -c 'cat <<EOF | while(read x) echo got $x
a
b
EOF' 2>&1 | sed 's/.*read.*/READ/'
rc -c 'fn f {cat <{echo inner}}; f; f'
rc -c 'fn f {echo $1 >[1=2]}; f x 2>&1; f y 2>/dev/null; echo done'
rc -c 'fn f {exec echo in f $*}; f 1; echo notreached'; echo "code=$?"
rc -c 'exec >f1 >[2=1]; echo a; echo b >[1=2]; exec >[1=2]' 2>&1; cat f1
rc -c 'exec </dev/null; cat; echo after'
rc -c 'exec 3; echo st=$status' 2>&1
rc -c 'true & wait; echo st=$status; false & wait; echo st=$status'
rc -c 'x=`{sleep 0.05; echo late} & wait; echo -$x-'
rc -c 'for(i in 1 2 3) {sleep 0.0$i; echo $i} & wait; echo done'
rc -c '{sleep 0.1; echo a} & {echo b} & wait; echo $status'
rc -c 'sleep 0.1 & apid1=$apid; sleep 0.2 & wait $apid1; echo st=$status; wait; echo st=$status'
rc -c 'exit 3 & wait $apid; echo st=$status'
rc -c 'echo a | {cat >/dev/null; exit 4}; echo st=$status'
rc -c '{exit 4} | cat; echo st=$status'
rc -c 'echo a | {cat >/dev/null; echo b} | {cat >/dev/null; echo c}; echo st=$status'
rc -c 'cat < nonexistent | cat; echo st=$status; echo after'; echo "code=$?"
rc -c 'cat | cat < nonexistent; echo st=$status; echo after'; echo "code=$?"
rc -c 'echo a | cat > nonexistent/x; echo st=$status; echo after'; echo "code=$?"
rc -c 'echo a >[1=5]; echo st=$status; echo after' 2>&1; echo "code=$?"
rc -c 'cat <[0=5]; echo st=$status; echo after' 2>&1; echo "code=$?"
rc -c 'echo a >[5=1]; echo st=$status'
rc -c 'echo a >[1=]; echo st=$status; echo b' 2>&1
rc -c 'echo a >[2=] | cat'
rc -c 'x=1; x=2 whatis x; whatis x; x=3 fn f {}; whatis x'
rc -c 'fn f {whatis x}; x=1 f; f'
rc -c 'x=1; fn f {x=2; g}; fn g {echo $x}; f; g'
rc -c 'x=1; fn f {g}; fn g {echo $x}; x=2 f; x=3 g'
rc -c 'x=(a b c); fn f {echo $x(2)}; x=(d e f) f'
rc -c 'fn f {*=(x y); echo $*; g}; fn g {echo $#* $*}; f a b c; echo $#*' 1 2
rc -c '*=(x y); echo $*; echo $1 $2 $3' 1 2 3
rc -c 'echo $#*; *=(); echo $#*; shift; echo $#* st=$status' a
rc -c '0=x; echo $0; echo $#0'
rc -c 'echo $*(2) $*(2-3) $#*(1)' a b c
rc -c 'x=(a b); echo $x^$x; echo $x(1)^$x(2)'
rc -c 'x=(); echo $#x^y; echo -$x^y-'; echo "code=$?"
rc -c 'x=(); y=(); echo $x^$y; echo -$x$y-'; echo "code=$?"
rc -c 'echo a^b^c; echo (a b)^c^(d e); echo (a b)^(c d)^(e f)'
rc -c 'echo a^(b c)^(d e f)'; echo "code=$?"
rc -c 'echo $x^$y' ; echo "code=$?"
rc -c 'x=y; echo $$x $$$x; echo $"$x'
rc -c 'x=(y z); y=1; z=2; echo $$x; echo $#$x'
rc -c 'x=y; y=(a b c); echo $$x(2); echo $$x(1 3)'
rc -c 'x=(a b); echo $x(1 2)(1)'; echo "code=$?"
rc -c 'x=1; echo $x(1); echo $x(2); echo $x(0)'
rc -c 'x=1; echo $x(1-)(1)'; echo "code=$?"
rc -c 'echo `{echo a b}(2)'
rc -c 'echo (a b c)(2)'; echo "code=$?"
rc -c 'echo $"*; echo $"1; echo $#"*' a b
rc -c 'echo $#$*' a
rc -c 'x=*.c; echo $x $"x; echo $x(1); ~ a.c $x; echo $status'
rc -c 'x=(a*); ~ abc $x; echo $status; ~ abc $"x; echo $status; y=a; ~ abc $y^*; echo $status'
rc -c '~ abc a^*; echo $status; ~ abc '"'"'a*'"'"'; echo $status; ~ a* '"'"'a*'"'"'; echo $status; ~ '"'"'a*'"'"' a*; echo $status'
rc -c 'echo '"'"'a'"'"'^*.c; echo *^.c'
rc -c 'x=(a b c); echo $x(3 2 1)'
rc -c 'echo -$"nonesuch-; echo $"nonesuch^x; echo $#"nonesuch'
rc -c 'x=(a b); echo $x^(1 2)^(3 4)'
rc -c 'x=1; y=2; if(~ $x 1 && ~ $y 2) echo both'
rc -c 'if(~ $x 1 || ~ $y 2) echo either; if not echo neither'
rc -c 'if(! ~ 1 2 && ! ~ 3 4) echo neither; if not echo some'
rc -c 'if(false; true) echo t; if(true; false) echo t; if not echo f'
rc -c 'if(x=1 true) echo $x; if not echo -$x-; if(x=1 false) echo $x; if not echo -$x-'
rc -c 'if(fn x {echo defined}) x; if not echo nope'
rc -c 'if(echo a > f2) cat f2'
rc -c 'if(echo hi) ; echo after'; echo "code=$?"
rc -c 'if() echo empty'; echo "code=$?"
rc -c 'if(
) echo empty'; echo "code=$?"
rc -c 'for(i in
a b) echo $i'; echo "code=$?"
rc -c 'for(i in a b
) echo $i'; echo "code=$?"
rc -c 'fn f
{echo body}
f'; echo "code=$?"
rc -c 'if(true)
echo next-line'
rc -c 'if(true) echo a
if not
echo b'; echo "code=$?"
rc -c 'while(false)
echo x
echo after'
rc -c 'switch(x)
{case x; echo x}'
rc -c 'switch(x) {
case x
echo x
}'
rc -c 'echo a &&
echo b ||
echo c'
rc -c 'echo a |
cat |
cat'
rc -c '{
echo a
echo b
} | cat'
rc -c '{ echo a ;
echo b }'
rc -c '(echo a; echo b)'; echo "code=$?"
rc -c 'echo (a;b)'; echo "code=$?"
rc -c 'echo ('"'"'a b'"'"' c)'
rc -c 'echo ()'
rc -c 'echo (())'
rc -c 'echo ((a) (b (c)))'
rc -c 'echo (a
b)'
rc -c 'x=(a
b); echo $#x'
rc -c 'echo a	b	c'
rc -c 'echo `{echo a
echo b}'
rc -c 'echo `{
echo a
}'
rc -c 'fn f {} ; f; echo st=$status'
rc -c 'fn f {
}
f; echo st=$status'
rc -c 'fn f { ; }; f; whatis f'
rc -c 'fn f {;;;echo x;;;}; f; whatis f'
rc -c '{;}; echo st=$status'; echo "code=$?"
rc -c '{}'; echo "code=$?"
rc -c '{ }; echo ok'; echo "code=$?"
rc -c 'echo a; {}; echo b'; echo "code=$?"
rc -c 'x=1; {x=2}; echo $x'
rc -c 'x=1; @{x=2}; echo $x'
rc -c 'x=1; {x=2} | cat; echo $x'
rc -c 'x=1; y=`{x=2}; echo $x'
rc -c 'cd d; @{cd sub}; pwd; @{cd sub; pwd}' | sed "s|$WORK|WORK|g"
rc -c 'cd d; {cd sub} | cat; pwd' | sed "s|$WORK|WORK|g"
rc -c 'cd d & wait; pwd' | sed "s|$WORK|WORK|g"
rc -c 'umask 0; umask & wait; umask'
rc -c 'x=1 & wait; echo -$x-'
rc -c 'fn f {echo f} & wait; whatis f'
rc -c 'echo `{cd d; pwd}; pwd' | sed "s|$WORK|WORK|g"
rc -c '`{cd d} ; pwd' | sed "s|$WORK|WORK|g"
rc -c 'echo `{exit 3}; echo st=$status; `{exit 4}; echo st=$status'
rc -c 'echo `{echo a; echo b >[1=2]} 2>/dev/null'
rc -c 'echo `{echo a >[1=2]}' 2>&1
rc -c 'echo `{echo a} >[2=1] | cat'
rc -c 'x=`{echo a; echo b}; echo $#x; x=`{echo a; echo b} | cat; echo $#x'
rc -c 'x=`{echo hi}; echo $x'
rc -c 'echo `{echo hi}^`{echo there}'
rc -c 'ifs=/; echo `{echo a/b/c}; ifs=(); echo `{echo a/b/c}; ifs=`'"'"''"'"'{printf "\t"}; echo `{printf "a\tb"}'
rc -c 'ifs='"'"''"'"'; echo $#ifs; echo `{echo a b} | od -c | sed 1q'
rc -c 'echo `'"'"'ab'"'"'{echo xaybz}; echo `'"'"'a b'"'"'{echo xaybz}; echo `(a b){echo xaybz}'
rc -c 'echo `x{echo xxaxx}'
rc -c 'echo ` {echo a b}; echo `{echo a b}'
rc -c 'echo `$ifs{echo a b}; x=b; echo `$x{echo abc}'
rc -c 'echo `*{echo a*b}'
rc -c 'echo `{echo a*b}; ls `{echo *.c}'
rc -c 'echo `{echo a b}^`{echo c d}'
rc -c 'echo `{printf "a"}; echo `{printf "a\n\n"}; echo `{printf "\na"}'
