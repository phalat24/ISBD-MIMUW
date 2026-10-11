# Analiza wyników pomiarów z zadań
## Latency
Rozwiązanie dostępne w pliku `latency_fized.rs`.
- Wykonanie programu dość dobrze się zrównolegla, do ~8 wątków, przy 16 nie widać już istotnego wzrostu wydajności. Przykładowo, w trybie `cold cache` i pliku rozmiaru 1GB (mieszczącym się w ramie) przy 1 wątku odczyt z dysku zajmuje średnio 44µs, przy 8 wątkach 66µs, a przy 16 wątkach 104µs. Efekt można wyjaśnić koniecznością systemowej synchronizacji wątków. 
- Tak, obrazują to wykresy `latency_cold_plot_log.png`, `latency_cold_plot_zoom.png`, `latency_warm_plot_log.png`, `latency_warm_plot_zoom.png`. 
- Nie, wręcz odwrotnie, w wyniku synchronizacji wątków lantecja zwiększa się z 44µs przy jednowątkowym wykonaniu do 104µs przy 16 wątkach.
- Posiadanie pliku w file cache nie wpływa na latencję, ale pozwala uniknąć czekania na odczyt pliku z dysku. W efekcie wykoniane programu z `warm cache` zajmuje wyraźnie mniej czasu.