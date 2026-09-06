rc -c 'echo héllo wörld; x=é; echo $x^x; echo -$#x-; ~ é ?; echo $status; echo 日本語'
rc -c 'echo '"'"'it'"''"'s'"'"'; echo '"'"''"'"'; echo '"'"' '"'"'; echo '"'"'a
b'"'"''
rc -c 'echo a	b; echo ` {echo c}; echo `	{echo d}'
rc -c 'x=1; echo $x^'"'"'-'"'"'^$x; echo $x'"'"'-'"'"'$x; echo $x-$x'
rc -c 'echo a;echo b;echo c'
rc -c 'echo a&echo b&wait'
rc -c 'echo {a}'; echo "code=$?"
rc -c 'echo a{b}'; echo "code=$?"
rc -c '{echo a}{echo b}'; echo "code=$?"
rc -c '{echo a};{echo b}'
rc -c '{echo a}
{echo b}'
rc -c 'if(true){echo a}'
rc -c 'if(true) {echo a} else {echo b}'; echo "code=$?"
rc -c 'while(false)'; echo "code=$?"
rc -c 'while(false) ; echo after'; echo "code=$?"
rc -c 'for(i in 1 2) ; echo after'; echo "code=$?"
rc -c 'if(true) ; echo after'; echo "code=$?"
rc -c 'fn f ; echo after'; echo "code=$?"
rc -c 'for(i in `{seq 1 5}) echo $i | cat'
rc -c 'n=0; for(i in `{seq 1 1000}) n=`{expr $n + 1}; echo $n' 
rc -c 'x=(); for(i in `{seq 1 200}) x=($x $i); echo $#x $x(200)'
rc -c 'echo `{seq 1 3}^x; echo x^`{seq 1 3}'
rc -c 'x=`{printf "a b\nc"}; echo $#x $x'
rc -c 'x=`{printf "\n\n a \n\n b \n"}; echo $#x $x'
rc -c 'x=`{printf ""}; echo $#x; x=`{printf " "}; echo $#x'
rc -c 'x=`'"'"'\n'"'"'{printf "a b\nc d"}; echo $#x; echo $x(1); x=`'"'"''"'"'{printf "a b\nc"}; echo $#x'
rc -c 'x=`{printf "a\0b"}; echo $#x $x' | od -c | sed 's/  */ /g'
rc -c 'x=`é{printf "aébéc"}; echo $#x $x'
rc -c 'x=`{cat $CASES/../test.rc}; echo $#x'
rc -c 'echo `{echo a; exit 3}; echo st=$status'
rc -c 'x=y `{echo cmd}; echo z'; echo "code=$?"
rc -c 'echo a >`{echo out}; cat out'
rc -c 'x=(a b); echo $x(1)(2)'; echo "code=$?"
rc -c '~ a a && echo yes || echo no'
rc -c 'true; ~ a b || echo failed $status'
rc -c 'echo -n; echo -n a; echo'
rc -c 'echo `{echo `{echo nested}}'
rc -c 'fn f {echo `{echo in-fn}}; f'
rc -c 'x=`{echo a}^`{echo b}; echo $x'
rc -c 'cat script.rc | rc -c '"'"'cat'"'"''
printf 'echo line1\necho line2\ncat\nline3\n' | rc
printf 'cat\nfoo\n' | rc
printf 'echo `{cat}\nfoo\n' | rc
printf 'echo a\ncat\n' > c1.rc; rc c1.rc < script.rc
yes | head -1000 > big.txt; printf 'head -1 big.txt\necho x\n' | rc
(printf 'echo start\n'; head -c 600 big.txt | tr -d '\n' | sed 's/y/ /g'; printf '\necho end\ncat\nrest\n') | rc
rc -c 'echo $#* $1' "a b" c
rc -c 'echo $*' 'a
b'
rc -c 'x=(a b); rc -c '"'"'echo $x(1)'"'"''
rc -c 'PATH=$PATH:/nonexistent whatis ls'
rc -c 'x=1; y=`{x=2; echo $x}; echo $x $y'
rc -c '{x=2} | cat; echo -$x-'
rc -c 'x=1 | echo -$x-'
rc -c 'x=1 true | echo -$x-'
rc -c '>f x=1; echo -$x-'
rc -c 'echo a >f | cat f; cat f'
rc -c 'if(x=1 true) echo $x; echo -$x-'
rc -c 'for(x=1 in a) echo'; echo "code=$?"
rc -c 'for(i in a b) x=$i; echo $x'
rc -c 'for(i in a b) x=$i echo $x; echo -$x-'
rc -c 'fn f {echo $*}; f a "b c" d'
rc -c 'x=a; whatis $x; whatis x$x'
rc -c 'echo `{echo x} `{echo y}'
rc -c 'ifs=(); echo `{printf "a b"} | od -c | sed 1q'
rc -c 'ifs=$nl; echo `{printf "a b\nc"}'
rc -c 'echo $ifs' | od -c | sed 1q
rc -c 'e=; echo -$e- $#e'
rc -c 'true; echo -$status-; whatis status'
rc -c 'x=1; fn f {echo $x; x=2}; f; echo $x'
rc -c 'fn f {x=2}; x=1 f; echo -$x-'
rc -c 'x=1; fn f {x=2}; x=3 f; echo $x'
rc -c 'fn f {echo $0}; f'
rc -c '. script.rc; fn f {echo $0}; f'
rc -c 'fn f {echo $#*}; f; f a; f (a b) c'
rc -c 'fn f {for(i) echo $i}; f x y'
rc -c 'fn f {exit 3}; f; echo after'; echo "code=$?"
rc -c 'fn f {@{exit 3}}; f; echo after $status'; echo "code=$?"
rc -c 'fn f {cat < nonexistent; echo in-f}; f; echo after'; echo "code=$?"
rc -c 'x=(a b c); fn f {echo $x(2)}; f'
rc -c 'echo 1 2 3 | rc -c '"'"'cat'"'"''
rc -c 'echo one > f; rc -c '"'"'cat < f'"'"''
rc -c 'echo a | cat | cat | wc -c'
rc -c 'echo abc | tr a-z A-Z | rev'
rc -c 'echo one | cat >f; cat f'
rc -c 'cat < f | cat'
rc -c 'exec cat < f'
rc -c '{cat} < f'
rc -c 'x=1 {cat} < f'
rc -c 'x=1 cat < f'
rc -c 'echo a | x=1 cat'
rc -c 'echo a | { x=1 cat }'
rc -c 'echo a | ! cat; echo $status'
rc -c 'echo a | @ cat; echo $status'
