rc -c 'fn f {echo hi $*; echo $#*}; f a b c; whatis f; fn f; whatis f' 
rc -c 'fn fact { if(~ $1 0) echo 1; if not { n=`{expr $1 - 1}; m=`{fact $n}; expr $1 \* $m } }; fact 5'
rc -c 'fn f {echo before $*; g x; echo after $*}; fn g {echo in g $*}; f 1 2'
rc -c 'fn f {x=local; echo $x}; x=global; f; echo $x'
rc -c 'fn f {y=1}; f; echo -$y-'
rc -c 'fn f {shift; echo $*}; f a b c'
rc -c 'fn f g {echo $0 called}; f; g'
rc -c 'fn f {echo a}; whatis f; fn f {echo b}; f'
rc -c 'fn f {echo $1}; fn f; f x; echo st=$status'
rc -c 'fn a {
	echo multi
	echo line
	for(i in 1 2) echo $i
}
whatis a
a'
rc -c 'fn cd {echo mycd $*}; cd /tmp; builtin cd /; pwd'
rc -c 'fn echo {builtin echo wrapped $*}; echo x'
rc -c 'builtin; echo st=$status'
rc -c 'builtin nonesuch; echo st=$status'
rc -c 'fn f {return}; f; echo $status'
rc -c 'fn f {echo $*} ; f
f `{echo a b}; f (x y) z'
rc -c 'fn f {
	if(~ $1 x) {
		echo isx
	}
	if not echo notx
}
f x; f y'
rc -c 'fn sigexit {echo bye}; echo main'
rc -c 'fn sigexit {echo bye $status}; exit 4'; echo "code=$?"
rc -c 'fn sigexit {echo bye; exit 9}; exit 4'; echo "code=$?"
