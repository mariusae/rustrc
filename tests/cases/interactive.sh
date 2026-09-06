# interactive mode driven from a pipe
printf 'echo a\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'x=1\necho $x\n' | rc -i 2>&1
printf 'if(true) {\necho in\n}\n' | rc -i 2>&1
printf 'for(i in a b)\necho $i\n' | rc -i 2>&1
printf 'echo a &&\necho b\n' | rc -i 2>&1
printf 'fn f {\necho x\n}\nf\n' | rc -i 2>&1
printf 'echo `{echo a\necho b}\n' | rc -i 2>&1
printf 'cat <<EOF\nhere\nEOF\necho after\n' | rc -i 2>&1
printf 'cat <<EOF <<EOF2\none\nEOF\ntwo\nEOF2\n' | rc -i 2>&1
printf 'echo a \\\nb\n' | rc -i 2>&1
printf 'echo a # comment \\\necho b\n' | rc -i 2>&1
printf 'prompt=(x y)\necho a\necho b\n' | rc -i 2>&1
printf 'prompt=(x y)\necho a \\\nb\n' | rc -i 2>&1
printf '\n\n\necho a\n' | rc -i 2>&1
printf '   \n# comment\necho a\n' | rc -i 2>&1
printf 'echo a; )\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'echo (\necho b\n' | rc -i 2>&1; echo "code=$?"
printf '{\necho b\n' | rc -i 2>&1; echo "code=$?"
printf 'if not echo x\necho y\n' | rc -i 2>&1; echo "code=$?"
printf 'if(true) echo a\nif not echo b\nif not echo c\n' | rc -i 2>&1; echo "code=$?"
printf 'echo (a b)^(c d e)\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'cat < nonexistent\necho after\n' | rc -i 2>&1; echo "code=$?"
printf '. nonexistent\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'x=1 | echo $x\n' | rc -i 2>&1
printf 'fn f {cat < nonexistent; echo in-f}\nf\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'exit 5\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'false\nexit\n' | rc -i 2>&1; echo "code=$?"
printf 'echo a' | rc -i 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -x 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -v 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -V 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -s 2>&1; echo "code=$?"
printf 'echo a\n' | rc -ii 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -I 2>&1; echo "code=$?"
printf 'echo a\n' | rc -I -i 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i script.rc 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -c 'echo c' 2>&1; echo "code=$?"
printf 'echo $#*\n' | rc -i - a b 2>&1; echo "code=$?"
printf 'echo $#* $1\n' | rc -i /dev/stdin a b 2>&1; echo "code=$?"
printf '. -i /dev/stdin\necho outer\n' | rc 2>&1; echo "code=$?"
printf 'flag i +\ncat < nonexistent\necho after\n' | rc 2>&1; echo "code=$?"
printf 'flag i -\ncat < nonexistent\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'echo a\n' | rc -i -e 2>&1; echo "code=$?"
printf 'false\necho b\n' | rc -i -e 2>&1; echo "code=$?"
printf 'x=`{echo a\n}\necho $x\n' | rc -i 2>&1
printf 'whatis prompt\nwhatis status\n' | rc -i 2>&1
printf 'prompt=()\necho $#prompt\n' | rc -i 2>&1
printf 'prompt=(a)\necho $#prompt\necho x\n' | rc -i 2>&1
printf 'prompt=(a b c)\necho x \\\ny\n' | rc -i 2>&1
printf 'prompt=(\x27\x27 \x27\x27)\necho x\n' | rc -i 2>&1
