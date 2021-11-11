input FACTORIAL;

### FACTORIAL ###
clear OUTPUT;
incr OUTPUT;
while FACTORIAL not 0 do;
while FACTORIAL not 1 do;
    copy OUTPUT to multOne;
    copy FACTORIAL to multTwo;

    clear multOut;
    while multOne not 0 do;
        copy multTwo to tmp;
        while multTwo not 0 do;
            incr multOut;
            decr multTwo;
        end;
        copy tmp to multTwo;
        decr multOne;
    end;
    copy multOut to OUTPUT
    decr FACTORIAL;
end;
decr FACTORIAL;
end;