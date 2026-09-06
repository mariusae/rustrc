rc -c 'echo $#* $1 $2 $3 $10' a b c d e f g h i j k
rc -c 'echo $*; shift; echo $*; shift 2; echo $*; shift; shift; echo -$*-; echo $status' a b c d
rc -c 'shift 1 2; echo $status'
rc -c 'x=(a b c); echo $x(1) $x(2)^y; echo $#x(1); echo $"x(2)'
rc -c 'x=(a b c); i=2; echo $x($i); j=(1 3); echo $x($j)'
rc -c 'x=(); echo -$x- $#x -$"x-; x=''; echo -$x- $#x -$"x-'
rc -c 'x=a; x=$x^b^$x; echo $x'
rc -c 'x=(a b); y=$x; echo $#y $y; y=$"x; echo $#y $y'
rc -c 'echo $pid' | grep -c '^[0-9][0-9]*$'
rc -c 'echo $#pid $#rcname $rcname'
rc -c 'echo -$cflag-'
rc -c 'echo $home $#home' | sed "s|$WORK|WORK|"
rc -c 'echo $#prompt; echo $prompt(1)'
rc -c 'echo $#ifs; echo $ifs | od -c | sed 1q'
rc -c 'path=(/bin /usr/bin); echo $path $PATH; PATH=/a:/b::/c; echo $path; PATH=; echo $#path -$PATH-; path=(); echo -$PATH- $#PATH'
rc -c 'PATH=/bin:/usr/bin; whatis ls'
rc -c 'path=(. /bin); shebang a'
rc -c 'x=1 rc -c '"'"'echo $x'"'"'; x=(a b) rc -c '"'"'echo $#x $x'"'"'; x=() rc -c '"'"'echo $#x'"'"'; x='"''"' rc -c '"'"'echo $#x'"'"''
rc -c 'fn f {echo fn from env $*}; rc -c '"'"'f x; whatis f'"'"''
rc -c 'fn f {echo one}; env | grep "^fn#"; x=(a b) env | grep "^x=" | od -c | sed 1q'
rc -c 'fn f {echo one}; fn g {echo two}; env | grep -c "^fn#"'
rc -c 'env | grep "^fn#f=" | wc -l'
rc -c 'x=y; whatis x; x=(a b); whatis x; x=(a '"'"'b c'"'"'); whatis x; x='"''"'; whatis x; x=('"''"' '"''"'); whatis x; x=a'"'"'b; whatis x; x=$x; whatis x; whatis nonesuch; echo st=$status; whatis nonesuch x; echo st=$status'
rc -c 'whatis'
rc -c 'whatis cd whatis eval exec exit shift wait . finit flag ulimit umask rfork builtin'
rc -c 'whatis echo; whatis /bin/echo; whatis ./script.rc; whatis nonexec' 
rc -c 'whatis "; whatis '"'"'a b'"'"'; whatis (a b)'
rc -c 'x=1; { x=2; echo $x }; echo $x'
rc -c 'x = 1; echo $x'
rc -c '(x) = 1; echo $x'
rc -c 'x=1=2; echo $x; echo a=b; echo =; echo a = b'
rc -c '1=x; echo $1' foo
rc -c 'x=(a b); echo $x(1-2) $x(2-2) $x(1-9) $x(0-1) $x(9-) $x(-1) $x(2-1)'
rc -c 'echo $#* $#1 $#2 $#nonesuch' a
rc -c 'x=a; $x=b; echo $a; y=(p q); $y=r; echo st=$status'
rc -c 'echo $(x)'; echo "code=$?"
rc -c 'x=(a b); echo $x^(1 2 3)'; echo "code=$?"
rc -c 'echo (a b)^(); echo after'; echo "code=$?"
rc -c 'echo ()^(); echo after'
rc -c 'x=(a b c); echo $x(2 1 3 1)'
rc -c 'echo $0' a b
rc -c '. src.rc a b; echo $x; echo $0 $#*' 
rc -c '. src.rc; . ./src.rc; . nonexistent.rc; echo after'; echo "code=$?"
rc -c '. script.rc; echo st=$status'
rc -c 'rc script.rc x y'
rc -c 'rc -c '"'"'echo $* $#* $0'"'"' a b c'
rc -c 'echo $status; . -i /dev/null; echo st=$status'
echo 'echo one; echo two' | rc -c '. /dev/stdin'
echo 'echo one' | rc -c '. -i /dev/stdin; echo st=$status'
rc -c 'flag i; echo $status; flag x; echo $status; flag x +; flag x; echo $status; flag x -; flag x; echo $status; flag xy +; echo $status; flag; echo $status; flag x y; echo $status'
rc -c 'flag x +; echo hi; flag x -; echo bye'
rc -c 'echo $#cdpath; cdpath=(d); cd sub; pwd; cd /; cd; pwd; cdpath=(. d); cd sub; pwd' | sed "s|$WORK|WORK|g"
rc -c 'cd nonexistent; echo st=$status; cd; echo st=$status; cd a b; echo st=$status; home=(); cd; echo st=$status; home=/nonexistent; cd; echo st=$status'
rc -c 'cd d; cd ..; pwd; cd ./d/sub; pwd; cd ../..; pwd' | sed "s|$WORK|WORK|g"
rc -c 'cdpath=(nonexistent d .); cd sub; pwd; cd e; pwd' | sed "s|$WORK|WORK|g"
