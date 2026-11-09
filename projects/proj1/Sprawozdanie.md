# Projekt nr 1 Sprawozdanie 

Testy programów wykonane na maszynie z linux oraz SATA SSD.

Dane do testów:
```bash
mkdir -p ~/isbd-data
head -c 256M /dev/urandom > ~/isbd-data/r256M
head -c 4G /dev/urandom > ~/isbd-data/r4G
```

Pomiar czasu `std::time::Instant`. Używam `sudo sysctl -w vm.drop_caches=3`.

## Latencja

Program teraz używa `sync_channel(1)` i czyta fragmenty pliku wielowątkowo. Nadal jeden wątek wykonuje obliczenia. Wątki alokują Vector<T> na stercie do którego czytany jest fragment pliku.

Wersja przed poprawką:

```bash
prog=latency M=64 THREADS=0 blocks=4194304 total_s=8.247788 read_avg_ns=1722 read_min_ns=459 read_max_ns=478470
```

```bash
prog=latency M=4096 THREADS=0 blocks=65536 total_s=5.139075 read_avg_ns=71962 read_min_ns=530 read_max_ns=5084681
```

```bash
prog=latency M=65536 THREADS=0 blocks=4096 total_s=1.770856 read_avg_ns=333686 read_min_ns=2380 read_max_ns=5271750
```

Wersja wielowątkowa (2, 8 wątków):

```bash
prog=latency M=64 THREADS=2 blocks=4194304 total_s=17.783589 read_avg_ns=2016 read_min_ns=480 read_max_ns=7799425
prog=latency M=64 THREADS=8 blocks=4194304 total_s=12.303164 read_avg_ns=2896 read_min_ns=490 read_max_ns=8062164
```

```bash
prog=latency M=4096 THREADS=2 blocks=65536 total_s=2.877827 read_avg_ns=81988 read_min_ns=620 read_max_ns=4904400
prog=latency M=4096 THREADS=8 blocks=65536 total_s=1.261784 read_avg_ns=131322 read_min_ns=670 read_max_ns=10231241
```

```bash
prog=latency M=65536 THREADS=2 blocks=4096 total_s=1.064305 read_avg_ns=474420 read_min_ns=2720 read_max_ns=4927800
prog=latency M=65536 THREADS=8 blocks=4096 total_s=0.648584 read_avg_ns=1094538 read_min_ns=2820 read_max_ns=7631645
```

Czas działania programu to mniej więcej (przy założeniu, że compute jest pomijalny):
`N/threads · (latency + heapallocation(M))`. Chociaż blok M=64 jest na tyle mały, że read() jest bardzo szybki w porównaniu do compute i przesyłania dużej liczby bloków przez kanał.

Zwiększenie liczby wątków pomaga gdy odczyt jest wąskim gardłem i trwa długo, wtedy dysk obsługuje kilka read() na raz, natomiast prowadzi do "lock contention" i dużej liczby alokacji przy małych blokach.

Każdy read() jest wolniejszy przy większej liczbie wątków. Natomiast throughput jest lepszy bo indywidualnie wolniejsze read() dzieją się wspólbierznie.

Przy użyciu file cache (program jeden po drugim):
```bash
prog=latency M=4096 THREADS=0 blocks=65536 total_s=0.411636 read_avg_ns=772 read_min_ns=610 read_max_ns=17710
```

```bash
prog=latency M=4096 THREADS=8 blocks=65536 total_s=0.427167 read_avg_ns=973 read_min_ns=740 read_max_ns=13800
```

Latencja spada ~100 krotnie. Nie czytamy wtedy z dysku tylko z RAM.

## Throughput

Plik zmapowany do pamięci, bez syscalli na każdy read().

Wersja przed poprawką:

```bash
prog=throughput mode=read M=512 size=4294967296 total_s=10.613165 mib_s=385.9
```

Z mmap:

```bash
prog=throughput mode=mmap M=512 size=4294967296 total_s=8.969049 mib_s=456.7
```

Czas działania programu w funkcji M to mniej więcej: `(plik/szybkość dysku)`, ale dla małych read() czas syscalli + compute może dominować.

Mmap zmniejszy obciążenie procesora przy małych read(). Czytanie tego samego fragmentu pliku również zmniejszy liczbę kopii do pamięci.

Przy dużych read() narzut syscalli spada.

File cache:

```bash
prog=throughput mode=read M=512 size=4294967296 total_s=10.352526 mib_s=395.7
```

```bash
prog=throughput mode=mmap M=512 size=4294967296 total_s=6.375030 mib_s=642.5
```

Read nadal płaci za syscalle i compute, mmap płaci za compute bo plik juz jest w RAMie.

## Pipeline

```bash
prog=pipeline mode=seq M=4096 K=1000 size=4294967296 total_s=101.949259 read_s=1.029435 compute_s=100.865554
prog=pipeline mode=pipe M=4096 K=1000 size=4294967296 total_s=102.062944 read_s=1.275404 compute_s=100.950234
prog=pipeline mode=seq M=65536 K=1000 size=4294967296 total_s=12.133417 read_s=0.535161 compute_s=11.594818
prog=pipeline mode=pipe M=65536 K=1000 size=4294967296 total_s=11.824624 read_s=0.611751 compute_s=11.603672
prog=pipeline mode=seq M=1048576 K=1000 size=4294967296 total_s=9.030792 read_s=2.871542 compute_s=6.158992
prog=pipeline mode=pipe M=1048576 K=1000 size=4294967296 total_s=8.932712 read_s=5.612770 compute_s=6.372888
prog=pipeline mode=seq M=16777216 K=1000 size=4294967296 total_s=13.058427 read_s=7.318804 compute_s=5.739553
prog=pipeline mode=pipe M=16777216 K=1000 size=4294967296 total_s=8.924996 read_s=8.757724 compute_s=5.825819
```

Czas działania wersji do poprawki to ~ `compute + read`, a poprawionej to ~ `max(compute, read)`.

Szukamy K_opt bez file cache:

```bash
prog=pipeline mode=pipe M=1048576 K=1 size=4294967296 total_s=8.981117 read_s=5.825971 compute_s=6.052005
prog=pipeline mode=pipe M=1048576 K=1000 size=4294967296 total_s=8.930730 read_s=5.634469 compute_s=6.350000
prog=pipeline mode=pipe M=1048576 K=4000 size=4294967296 total_s=8.942873 read_s=5.099459 compute_s=7.385603
prog=pipeline mode=pipe M=1048576 K=8000 size=4294967296 total_s=8.999611 read_s=3.818693 compute_s=8.627290
prog=pipeline mode=pipe M=1048576 K=10000 size=4294967296 total_s=9.402340 read_s=0.834354 compute_s=9.339087
```

Zatem dla mojego dysku K_opt to ~ 9000.

Teraz z file cache:

```bash
prog=pipeline mode=pipe M=1048576 K=1 size=4294967296 total_s=5.613562 read_s=0.186465 compute_s=5.608335
```

Compute od razu dominuje gdy plik jest w file cache, K_opt to 1.

Więcej wątków po stronie czytania nic nie da, bo 1 wątek już czyta z ~ prędkością dysku, a gdy plik jest w file cache read() jest pomijalny w porównaniu do compute. Jeżeli compute >= read to więcej wątków compute pomoże, xor można liczyć wspólbierznie.
