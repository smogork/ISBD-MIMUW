# Laboratorium 1 - Pomiary podstawowych wielkości przy odczytach plików

- [Laboratorium 1 - Pomiary podstawowych wielkości przy odczytach plików](#laboratorium-1---pomiary-podstawowych-wielkości-przy-odczytach-plików)
  - [Problem - Obliczenie sumy XOR funkcji skrótu bloku z każdego pliku](#problem---obliczenie-sumy-xor-funkcji-skrótu-bloku-z-każdego-pliku)
  - [Programy do poprawienia](#programy-do-poprawienia)
    - [Latencja - losowy odczyt](#latencja---losowy-odczyt)
    - [Przepustowość - mechanizm zero-copy](#przepustowość---mechanizm-zero-copy)
    - [Przeplot obliczeń oraz operacji IO - pipelining](#przeplot-obliczeń-oraz-operacji-io---pipelining)
  - [Jak wykonać pomiary?](#jak-wykonać-pomiary)
    - [Pomiar czasu](#pomiar-czasu)
    - [Mechanizm file cache](#mechanizm-file-cache)
    - [Dobór rozmiaru plików](#dobór-rozmiaru-plików)
  - [Sprawozdanie z pomiarów](#sprawozdanie-z-pomiarów)
  - [Testy po oddaniu projektu](#testy-po-oddaniu-projektu)
  - [Materiały do przeczytania](#materiały-do-przeczytania)


Celem laboratorium jest zbadanie 2 wielkości związanych z oprogramowaniem zajmującym się odczytami z dysków:
* latencja odczytu
* przepustowość transmisji danych

Istnieją dwa typy systemów DBMS, które optymalizuje się pod jedno z tych dwóch zastosować:
* OLTP (latencja)
* OLAP (przepustowość)

W ramach tego laboratorium będziemy badać program, który wykorzystuje odczyt z dysku i oblicza funkcję skrótu danego pliku.

## Problem - Obliczenie sumy XOR funkcji skrótu bloku z każdego pliku

Wszystkie poniższe programy będą dzieliły wczytany plik na bloki o rozmiarze `M` bajtów.
Ich celem jest obliczenie funkcji skrótu każdego bloku z osobna i połączenie go w jeden finalny wynik.

$$R(F) = H(B_1) \oplus H(B_2) \oplus \dots \oplus H(B_N),$$

gdzie $N$ to ilość bloków, na które podzielony jest plik $F$.
Blok $B_i$ to ciągły kawałek pliku rozpoczynający się na bajcie $i \cdot M$ i kończący się na $(i + 1)M - 1$.
Powyższa funkcja skrótu jest przemienna i łączna, co daje nam taki sam wynik niezależnie od kolejności odczytanych bloków w pliku.

Aby dodać parametr sterujący ilościa obliczeń programu, funkcja hashująca to wielokrotne wykonanie funkcji MD5 na wyjściu poprzedniej iteracji

$$H(B,K) = \underbrace{MD5(MD5( \dots MD5(B)))}_\text{K razy}.$$

Ta technika to tzw. *Key Stretching*.
Pozwala ona parametryzować jak trudno jest wykonać atak brute-force na hash zwiększając wartość $K$.

W taki sposób problem został podzielony na dwie części:
1. *read* - operacja przeczytania bloku z dysku (*producent*)
2. *compute* - operacja wyliczenia funkcji $H(B_i)$ (*konsument*)

## Programy do poprawienia

W folderze `src` można znaleźć podstawowe implementacje rozwiązania problemu wyliczania funkcji skrótu.
Każdy z nich ma zaprezentować pewien sposób radzenia sobie z latencją i przepustowością.
Twoim zadaniem jej poprawienie podstawowych implementacji i zbadanie jak to wpłynęło na zachowanie programu.
Wyniki obserwacji zmienionego kodu należy opisać w sprawozdaniu końcowym.

### Latencja - losowy odczyt

Pierwszy program rozwiązuje problem, czytając bloki w losowej kolejności.
Używa on do tego synchronicznego `read()` i małych bloków ($M=512B$).
W tym momencie czas wykonania obliczeń to mniej więcej $T \approx N * latency$.

Parametr $K=1$ jest ustalony na stałe.
Rozmiar bloku $M$ jest parametrem programu.
Każda operacja *read* potrzebuje czasu na wysłanie zlecenia i wysłanie danych do procesora.
Operacja *compute* jest pomijalnie krótka względem czytania z dysku.

Popraw ten program dodając wiele wątków, które wykonują operację `pread()` z wcześniej obliczonym offsetem.
Każdy wynik przekaż do jednego wątku wykonujący operację *compute*.

Należy poruszyć w sprawozdaniu:
* Czas działania programu w funkcji M i ilości wątków.
* Czy zwiększanie ilości wątków zmniejszy czas wykonania programu?
* Czy zwiększenie ilości wątków zmniejszy latencję operacji odczytywania danych?
* Jak wpłynie na latencję posiadanie pliku w pamięci systemu operacyjnego (file cache)?

### Przepustowość - mechanizm zero-copy

Drugi program rozwiązuje problem, czytając plik sekwencyjnie od początku do końca.
Używa on do tego synchronicznego `read()` i małych bloków ($M=512B$).
W tym momencie czas wykonania programu to suma:
* przepustowość dysku
* czasu przekazania sterowania do systemu operacyjnego (syscall - `read`)
* kopiowanie danych do przestrzeni pamięci procesu

Parametr $K=1$ jest ustalony na stałe.
Rozmiar bloku $M$ jest parametrem programu.

Popraw ten program mapując plik do pamięci wirtualnej procesu.
Wtedy narzut kopiowania i oddawanie sterowania do systemu operacyjnego zostanie całkowicie pominięty.
Użyj do tego funkcji *mmap()*.

Należy poruszyć w sprawozdaniu:
* Czas działania programu w funkcji M (programu przed poprawką oraz po poprawce)
* Czy jest jakiś schemat odczytywania pliku, gdzie mmap zmniejszy obciążenie procesora ponad read?
* Jak wpłynie na latencję posiadanie pliku w pamięci systemu operacyjnego (file cache)?

### Przeplot obliczeń oraz operacji IO - pipelining

Trzeci program rozwiązuje problem, także czytając sekwencyjnie, jednak tym razem K jest duże ($K=1000$).
Używa on do tego synchronicznego `read()` i średnich bloków ($M \approx 1M$).
W tym scenariuszu czas wykonania programu to suma operacji IO (przepustowość) oraz czas wykonania operacji *compute*.

Rozmiar bloku $M$ oraz ilość iteracji $K$ są parametrami programu.
Popraw powyższy program tworząc dwa wątki: czytający z pliku oraz obliczający funkcję skrótu.
Połącz wątki kanałem o ograniczonej wielkości (`sync_channel`).

W tej architekturze czas wykonania zmieni się z sumy czasów *read* oraz *compute* na maximum z dwóch.
Na ogół dla małych K, dysk jest wolniejszy niż obliczanie funkcji skrótu.
Oznacza to, że istnieje takie $K_{opt}$, gdzie w zakresie $[1, K_{opt}]$ program nie zwalnia.
Natomiast każde $K > K_{opt}$ powoduje zwiększenia czasu działania.

Należy poruszyć w sprawozdaniu:
* Czas działania programu w funkcji $M$ i $K$
* Znajdź $K_{opt}$ dla $M=1M$ dla twojego komputera. Rozważ plik w całości mieszczący się w pamięci operacyjnej oraz taki, który należy ciągle transmitować z dysku.
* Czy dodanie większej ilości wątków po stronie czytania lub funkcji hashującej zmieni cokolwiek?

## Jak wykonać pomiary?

### Pomiar czasu
Pierwszy krok to precyzyjny pomiar czasu. Przy małych rozmiarach danych wejściowych operujemy na wielkościach mikrosekund.
Natywną funkcją dla linuxa jest użycie funkcji `clock_gettime` i podanie parametrów dla zegara monotonicznego.
W ruscie jednak istnieje mechanizm `std::time::Instant`. Dzięki niemu można poznać dokładnie, ile czasu trwało wykonanie wybranych operacji.

Przez nieodłączny szum związany z działaniem systemu operacyjnego i innych procesów warto skupić się na minimum z wielu pomiarów.
Taki pomiar daje gwarancję powtarzalności.

### Mechanizm file cache

System operacyjny posiada mechanizm przetrzymywania już przeczytanych plików w pamięci operacyjnej. 
Wspomaga to wielokrotne odczytywanie tego samego miejsca na dysku.
Strategia cachowania opiera się na algorytmie LRU i system operacyjny wywłaszcza najdawniej używane strony pamięci.
Mówimy wtedy, że system jest pod *presją pamięciową* (*memory pressure*) i musi używać czasu procesora do zwalniania pamięci reaktywnie.

Oznacza to, że wasze programy mogą kończyć się zaskakująco szybko, ponieważ po wczytaniu danych do cache, pomiary będą wskazywać na operacje w pamięci operacyjnej.
Aby powiedzieć kernelowi, żeby wyczyścił cały cache, należy użyć polecenia
```bash
sudo sysctl -w vm.drop_caches=3
```

Przy otwarciu pliku z dodaniem flagi `O_DIRECT` można całkowicie pominąć mechanizm cachowania. To także powinno ułatwić testy (`open(2)`).

### Dobór rozmiaru plików

W ramach tego laboratorium rozmiar pliku będzie mieć duże znaczenie.
Wszystkie programy w ramach tego laboratorium powinny być gotowe na przeczytanie plików większych niż pamięć operacyjna systemu.
Jednak testowanie tak dużych plików to często strata czasu, ponieważ wraz ze wzrostem rozmiaru danych więcej czasu zajmuje także część obliczeniowa.

W systemie operacyjnym istnieje mechanizm ograniczania dostępniej pamięci i proponuje z niej skorzystać do testowania ograniczeń i zachowania programów pod presją pamięciową.
```bash
sudo systemd-run --scope -p MemoryMax=512M binary_path
```
To polecenie ograniczy nie tylko rozmiar pamięci procesu, ale także file cache.

## Sprawozdanie z pomiarów

Jako wynik z tego laboratorium oczekuję dostarczenia repozytorium z rozwiązaniami trzech problemów oraz sprawozdanie z pomiarów na swoim komputerze.
Ocena będzie sumą z programów (10pkt.) oraz wniosków ze sprawozdania (10pkt.).

## Testy po oddaniu projektu
Na laboratorium nr 3 wykonamy test wybranych programów na trzech różnych technologiach dysków:
* Dysk talerzowy (HDD)
* Dysk SATA SSD
* Dysk NVMe SSD

## Materiały do przeczytania

* Instrukcje użytkownika POSIX(3p) oraz syscalle(2)
  * `read(2)` + `read(3p)`
  * `open(2)`
  * `fseek(3p)`
  * `fstat(3p)`
  * `mmap(2)` + `mmap(3p)`
  * `munmap(3p)`
  * `msync(3p)`
  * `sys_mman.h(0p)`
  * `clock_gettime(3p)`
* Odnośniki do dokumentacji biblioteki standardowej Rusta
  * [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html)
  * [`memmap`](https://docs.rs/memmap/latest/memmap/struct.Mmap.html)
  * [`std::sync::mpsc::sync_channel`](https://doc.rust-lang.org/beta/std/sync/mpsc/fn.sync_channel.html)
