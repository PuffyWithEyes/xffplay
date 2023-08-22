# XFFPLAY
Отрисовка элементов интерфейса происходит с помощью **X11 API**. Получение сообщений происходит благодаря **Linux API**, используя крейт `msg`.
## Ветки и проблемы
В данном репозитории имеются две ветки, имеющие проблемы.
>main
Первая **`main`** ветка отрисовывает элементы на процессе `ffplay` в бесконечном цикле, с задержкой в 3мс - что очень обыстро. К сожалению, из-за постоянного обновления с помощью метода `flush()` в соединении `conn`, появляются большие проблемы с производителбностью.
>trigger-loop
Вторая **`trgger-loop`** ветка отрисовывает элементы на процессе `ffplay` в бесконечном цикле, в котором ожидается взаимодействие с окном и происходит отрисовка элементов. К сожалению, из-за правил отрисовки `ffplay` и правил отрисовки `X11` получается так, что отрисованные элементы оказываются снизу полученной картинки с камеры.
### Получение сообщений от другого процесса
>>main и trigger-loop
Получение сообщений, как ранее говорилось - происходит благодаря **Linux API** - крейт `msg`. Получение сообщений происходит в бесконечном цикле потока в `main` функции, где внешняя функция `msgrcv` является блокируещей. Та в свою очередь принимает **ID**, которое генерируется с помощью функций `ftok`, которая в свою очередь генерирует ключ с помощью файла по пути `/etc/xffplay/token.txt`, который генерируется благодаря установщику installer.bash. В директории `child` данного репозитория находится простой пример на **C**, который генерирует случайные числа и отсылает их с помощью `msgsnd` в бесконечном цикле.
### Отрисовка элементов интерфейса
>>main и trigger-loop
Отрисовка в обоих ветках происходит с помощью функций `draw_line` и `draw_text`. В моем коде, в функции `draw_text` есть использование строки `"6x13"` - которая является шрифтом **X11**. Чтобы получить список шрифтов **X11** в системе, можно прописать в терминал:
```bash
xlsfonts
```
Цвет текста так же зависит от полученного числа
### Установка
Установка происходит всего-лишь с помощью установщика:
```bash
sudo bash installer.bash
```
### Подробнее о проблемах
>trigger-loop
Эта ветка имеет так же проблему с получением текста от приложения-примера, когда оно приходит - текст не обновится, пока не будет произведена какая-либо манипуляция над окном. Фикс не имеет смысла, так как праила отрисовки **Х11** менять очень тяжело и долго.

*P.S.: Будет попытка сделать подобное приложение, используя wayland API*

