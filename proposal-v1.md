# bimhd.sk
### budget imhd clone

### Features etapa 1
- zobrazovanie trasy od zastavky A po B
  - od konkretneho casu
  - celkovo (bez casu)
- ETA (bez pesej chodze)
- vylistovat spoje
  - pre zastavku
    - vylistovat odchody pre spoje na danej zastavke
- moznost vybrat si konkretne typy vozidiel
- pre konkretnu GPS polohu vylistovat najblizsie zastavky
- cachovanie vysledkov (in memory / fs)
- DEV metriky - vramci responses vratit kolko cely proces trval

### Features etapa 2
- rate limiting
- ETA s pesou chodzou (+20% overhead) (bude to ratat vzdusnou ciarou :p)
- pridat nejaky auth (JWT)

### Features etapa 3
- mozno nejaky frontend?
- dokumentacia pre developerov (extremne nice to have)

# Detaily
- Rest api (Actix framework)

# TODO
- spravit repozitar
- napisat kostalovi aky ma byt minimalny/maximalny rozsah
- spytat sa kamosa na realtime data o spojoch