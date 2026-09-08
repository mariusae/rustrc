rc -Z; echo "code=$?"
rc -c; echo "code=$?"
rc -cc 'echo x'; echo "code=$?"
rc -ii -c 'echo x'; echo "code=$?"
rc -e -c 'echo a; false; echo b'; echo "code=$?"
rc -e -c 'true && false; echo b'; echo "code=$?"
rc -e -c 'if(false) echo x; echo b'; echo "code=$?"
rc -e -c '! true; echo b'; echo "code=$?"
rc -e -c '~ a b; echo b'; echo "code=$?"
rc -e -c 'false | true; echo b'; echo "code=$?"
rc -e -c '@{false}; echo b'; echo "code=$?"
rc -e -c '{false}; echo b'; echo "code=$?"
rc -e -c 'x=`{false}; echo b'; echo "code=$?"
rc -e -c 'fn f {false; echo in f}; f; echo b'; echo "code=$?"
rc -e -c 'false || true; echo b'; echo "code=$?"
rc -x -c 'echo a; x=1; echo $x; for(i in 1 2) echo $i; fn f {echo f $*}; f z; echo `{echo bq}; false; echo done' 2>&1
rc -s -c 'echo a; false; echo b; exit 3' 2>&1; echo "code=$?"
rc -v -c 'echo a; echo b' 2>&1
rc -V -c 'echo a; echo b' 2>&1
printf 'echo a\necho b\n' | rc -v 2>&1
printf 'echo a\necho b\n' | rc -V 2>&1
printf 'echo a\necho b\n' > v.rc; rc -v v.rc 2>&1; rc -V v.rc 2>&1
rc -I -c 'echo a'
printf 'echo hi\nfalse\necho $status\n' | rc -i 2>&1; echo "code=$?"
printf 'echo hi\n' | rc -i -c 'echo cmd' 2>&1; echo "code=$?"
printf 'echo hi\n' | rc -i x.rc 2>&1; echo "code=$?"
printf 'echo a )\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'cat < nonexistent\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'echo (a b)^(c d e)\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'prompt=(one two); echo hi\necho there\n' | rc -i 2>&1; echo "code=$?"
printf 'prompt=(one two); echo hi \\\n more\ncat <<EOF\nx\nEOF\n' | rc -i 2>&1; echo "code=$?"
printf 'prompt=(); echo hi\n' | rc -i 2>&1; echo "code=$?"
printf 'prompt=one; echo hi\n' | rc -i 2>&1; echo "code=$?"
printf 'echo hi\n' | rc -i -e 2>&1; echo "code=$?"
printf 'exit 3\n' | rc -i 2>&1; echo "code=$?"
printf 'false\n' | rc -i 2>&1; echo "code=$?"
printf 'echo hi\n' | rc -i -s 2>&1; echo "code=$?"
printf 'false\n' | rc -i -s 2>&1; echo "code=$?"
rc -l -c 'echo login' 2>&1; echo "code=$?"
mkdir -p lib; echo 'echo profile' > lib/profile; rc -l -c 'echo login'; rc -c 'echo nologin'; echo 'echo x' | rc -l; echo 'echo y' | rc -l -i 2>&1
echo 'echo rcrc $#*; fn fromrc {echo fromrc}' > lib/rcrc; rc -c 'echo c; fromrc'; echo 'echo stdin' | rc; rc ./script.rc a; rc -l -c 'echo login'; echo 'echo i' | rc -i 2>&1; rc -m mymain; rm lib/profile lib/rcrc
rc -m; echo "code=$?"
echo 'echo custom rcmain $*; echo $#*' > mymain; rc -m mymain a b; echo "code=$?"
rc -m nonexistent -c 'echo x'; echo "code=$?"
rc -p -c 'echo $path'
rc -d -c 'echo d'
rc -r -c 'echo r' 2>&1 | sed 's/cycle [0-9A-F]* [0-9]*/cycle ADDR PC/' | grep -c Xsimple
rc -D -c 'echo a b | c >[2=1] && ! d; for(i in x) if(~ $i y) z; if not w' 2>&1
rc -D -c 'x=y a; fn f {b; c}; switch(a){case b; c}; `{d}; <{e}; @f; {g} >h; i <<EOF
EOF
' 2>&1
rc -DY -c 'echo a' 2>&1
rc -S -c 'echo S'
rc -c 'echo a' -c 'echo b'; echo "code=$?"
rc - -c 'echo dash'; echo "code=$?"
rc -- -c 'echo dashdash'; echo "code=$?"
rc -c 'echo x' -e; echo "code=$?"
rc -ec 'echo ec'; echo "code=$?"
rc -c'echo attached'; echo "code=$?"
rc -m mymain -c 'echo c'; echo "code=$?"
rc -ic 'echo ic' 2>&1; echo "code=$?"
rc -xe -c 'echo xe' 2>&1
rc -c 'echo x' script.rc; echo "code=$?"
rc script.rc -c 'echo y'; echo "code=$?"
rc -c 'flag e +; false; echo after'; echo "code=$?"
rc -c 'flag e +; eval false; echo after'; echo "code=$?"
rc -e -c 'flag e -; false; echo after'; echo "code=$?"
rc -c 'flag x +; echo traced' 2>&1
rc -c 'flag i +; echo $status; cat < nonexistent; echo after'; echo "code=$?"
rc -c 'flag i; echo st=$status; flag I; echo st=$status; flag c; echo st=$status; flag c -; echo -$cflag-; flag l; echo $status; flag p; echo $status'
rc -c 'flag Y +; echo $status'
