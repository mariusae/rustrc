# environment round trips between parent and child rc
rc -c 'x=(a b c); fn f {echo f $*}; fn g {echo `{echo in g}; x=(1 2)}; rc -c '"'"'echo $x $#x; f 1; g; echo $x; whatis f g'"'"''
rc -c 'fn f {echo '"'"'quoted '"'"''"'"'string'"'"''"'"' $"x}; rc -c '"'"'whatis f; f'"'"''
rc -c 'fn f {if(~ $1 a) echo A; if not echo notA}; rc -c '"'"'f a; f b; whatis f'"'"''
rc -c 'fn f {for(i in $*) {echo $i; if(~ $i x) echo X}}; rc -c '"'"'f x y'"'"''
rc -c 'fn f {echo a | tr a-z A-Z; @{echo b}; ! false; ~ a a}; rc -c '"'"'whatis f; f; echo $status'"'"''
rc -c 'fn f {echo >[2=1] hi >/dev/null <<EOF
x
EOF
}; whatis f; rc -c '"'"'whatis f'"'"''
rc -c 'fn f {x=1 y=2 echo $x $y; z=3}; rc -c '"'"'whatis f; f; echo $z'"'"''
rc -c 'fn f {switch($1){case a; echo A; case *; echo other}}; rc -c '"'"'whatis f; f a; f b'"'"''
rc -c 'fn f {while(! ~ $#* 0) {echo $1; shift}}; rc -c '"'"'f 1 2 3; whatis f'"'"''
rc -c 'fn f {echo $#*}; fn g {f $* $*}; rc -c '"'"'g a b'"'"''
env x=a"$(printf '\001')"b rc -c 'echo $#x $x'
env x= rc -c 'echo $#x -$x-'
env 'a(b)=1' rc -c 'echo -$a-; env | grep "^a(" | wc -l'
env 'fn#h={echo from fn env}' rc -c 'h; whatis h; env | grep "^fn#h" | wc -l'
env 'fn#h={echo bad' rc -c 'echo after; whatis h' 2>&1; echo "code=$?"
env 'fn#h=' rc -c 'echo after; whatis h' 2>&1; echo "code=$?"
env 'fn#h' rc -c 'echo after' 2>&1; echo "code=$?"
env 'fn#h={echo one}' 'fn#i={echo two}' rc -c 'h; i; finit; h; i; echo st=$status'
rc -c 'x=1; rc -c '"'"'x=2; rc -c '"'"'"'"'"'"'"'"'echo $x'"'"'"'"'"'"'"'"''"'"'; echo $x'
rc -c 'path=(/nonexistent /bin); rc -c '"'"'echo $path; echo $PATH'"'"''
rc -c 'PATH=/usr/bin:/bin; env | grep "^PATH=" ; env | grep "^path=" | od -c | sed 1q'
rc -c 'x=(a b); env | grep "^x=" | od -c | sed 1q; x=(); env | grep -c "^x="'
rc -c 'x=1 env | grep "^x="; echo -$x-'
rc -c 'x=1; x=2 env | grep "^x="; echo $x'
rc -c 'fn x {echo fn}; x=1; env | grep "^x=" ; env | grep "^fn#x="'
rc -c 'ifs=(a b); env | grep "^ifs=" | od -c | sed 1q'
rc -c 'status=x; env | grep "^status="; true; env | grep -c "^status="'
rc -c 'env | grep "^pid=" | wc -l; env | grep "^prompt=" | wc -l; env | grep "^cflag=" | wc -l; env | grep "^rcname=" | wc -l'
rc -c 'env | grep -c "^PLAN9="; echo $#PLAN9'
env -i PATH=$PATH HOME=/tmp rc -c 'echo $home; echo $#PLAN9; echo $#prompt; echo $#ifs'
env -i PATH=$PATH rc -c 'echo -$home-; echo $#path'
env -i rc -c 'echo -$path- $#PATH' 2>&1; echo "code=$?"
env -i PATH= rc -c 'echo -$path- -$PATH- $#path; echo hi' 2>&1; echo "code=$?"
env -i PATH=: rc -c 'echo $path' 2>&1
env -i PATH=/bin: rc -c 'echo $path; echo $PATH' 2>&1
env HOME=/nonexistent rc -c 'echo $home; cd; echo $status'
env -u HOME rc -c 'echo $home'
env home=/tmp HOME=/ rc -c 'echo $home'
env 'x=a b' rc -c 'echo $#x; echo $x'
env "$(printf 'x=a\nb')" rc -c 'echo $#x; echo $x'
