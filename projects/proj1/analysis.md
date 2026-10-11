# Analiza wyników pomiarów z zadań

## Latency

Rozwiązanie dostępne w pliku `latency_fized.rs`.
- Wykonanie programu dość dobrze się zrównolegla, do ~8 wątków, przy 16 nie widać już istotnego wzrostu wydajności. Przykładowo, w trybie `cold cache` i pliku rozmiaru 1GB (mieszczącym się w ramie) przy 1 wątku odczyt z dysku zajmuje średnio 44µs, przy 8 wątkach 66µs, a przy 16 wątkach 104µs. Efekt można wyjaśnić koniecznością systemowej synchronizacji wątków. 
- Tak, obrazują to wykresy `latency_cold_plot_log.png`, `latency_cold_plot_zoom.png`, `latency_warm_plot_log.png`, `latency_warm_plot_zoom.png`. 
- Nie, wręcz odwrotnie, w wyniku synchronizacji wątków lantecja zwiększa się z 44µs przy jednowątkowym wykonaniu do 104µs przy 16 wątkach.
- Posiadanie pliku w file cache nie wpływa na latencję, ale pozwala uniknąć czekania na odczyt pliku z dysku. W efekcie wykoniane programu z `warm cache` zajmuje wyraźnie mniej czasu.

## Throughput

Rozwiązanie dostępne w pliku `throughput_fixed.rs`.
- W obu wersjach czas rośnie liniowo z rozmiarem pliku, a wersja z `mmap` jest ok. 25% szybsza od wersji z `read`. W przypadku wykonania na moim komputerze obie wersje są ograniczone przez liczenie MD5 na jednym rdzeniu (~0,75GB/s), a nie przez dysk (odczyt sekwencyjny ~5GB/s). Dla plików większych niż limit pamięci (2GB) przepustowość spada tylko o 9-14%. Obrazują to wykresy `throughput_comparison_plot_zoom.png` i `throughput_comparison_plot_log.png`.
- Tak, sekwencyjny odczyt małymi blokami pliku znajdującego się w file cache. Dysk nie gra wtedy roli, a koszt `read` to głównie wywołanie systemowe i kopiowanie danych, które `mmap` eliminuje. Ogólnie im mniejszy blok M, tym więcej wywołań `read` i tym większa przewaga `mmap`. 

- Wpływ jest niewielki, tryb `cold cache` jest wolniejszy tylko o 4-7%. Przy odczycie sekwencyjnym jądro stosuje readahead, ładując do pamięci ramki wprzód sekwencyjnego wykoniania, ukrywając latencję.


## Pipeline

Rozwiązanie dostępne w pliku `pipeline_fixed.rs`.
- Czas rośnie liniowo z rozmiarem pliku, a od K zależy słabo: pierwsza iteracja MD5 liczona jest z całego bloku 1MB. Wyniki `cold cache` i `warm cache` są praktycznie identyczne dla każdego K, także dla plików większych niż limit pamięci (10GB: 11905ms wobec 11906ms dla K=1), co oznacza, że dysk w ogóle nie jest wąskim gardłem. Rozdzielenie odczytu i obliczeń na dwa wątki zmienia czas z sumy na maksimum: dla pliku 1GB i K=1000 wersja bazowa `pipeline.rs` zajmuje 1519ms (`cold`) i 1409ms (`warm`), a `pipeline_fixed.rs` 1337ms w obu trybach, czyli tyle, ile same obliczenia. Obrazują to wykresy `pipeline_cold_plot_zoom.png`, `pipeline_cold_plot_log.png`, `pipeline_warm_plot_zoom.png`, `pipeline_warm_plot_log.png`.
- Do znalenienia K_opt służy skrypt `find_k_opt.py`: mierzy czas samego odczytu T(0) (K=0, bez hashowania), a następnie szuka największego K, dla którego czas nie przekracza T(0) o więcej niż 5%. Na moim komputerze K_opt nie istnieje w żadnym z przypadków: dla pliku 1GB T(0)=211ms przy odczycie z dysku (~4,7GB/s) i ok. 110ms z pamięci, a już K=1 trwa 1200ms.
- Dodanie wątków czytających nic nie zmieni, bo wątek czytający i tak przez większość czasu czeka na zwolnienie miejsca w kanale. Dodanie wątków liczących funkcję skrótu przyspieszyłoby tutaj program.