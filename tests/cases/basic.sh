rc -c 'echo hello world'
rc -c 'x=(a b c); echo $x(2) $#x $"x; echo $x(1 3) $x(2-) $x(2-3) $x(5) $x(0) $x(3-1) $x(1-1)'
rc -c 'echo `{echo a b; echo c}; echo `{echo}; echo x`{echo y}z'
rc -c 'ifs=x; echo `{echo axbxc}; echo `''''{echo abc}; ifs=(); echo `{echo a b}'
rc -c 'echo a^(1 2) (x y)^(1 2); echo -$x- ; echo $$x'
rc -c 'x=(1 2 3); y=(a b c); echo $x^$y; echo $x^-^$y'
rc -c 'echo a''b''c; echo a'"'"'b'"'"'c; echo '"'"''"'"''"'"''"'"'' 
rc -c 'echo a b#comment
echo c'
rc -c 'echo one \
two'
rc -c 'echo $nonesuch; echo $#nonesuch; echo -$"nonesuch-'
