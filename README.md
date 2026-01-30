High performance flexible rust binary tcp server.

You can find usage examples in the examples module.

The project in very raw, if you have bugs, issues or wishes. Feel free to create issues or contribute

P.S:
This server is not designed to transfer a big chunks of data. But it can do this task.
You will get a big delays on doing this. For 250 mb it's around 224556 microseconds.

For small chunks of e.g before 50 kilobytes, you will get a little delays between messages, ]
about 300-400 microseconds if traffic is not constant. And 40-80 microseconds if traffic is constant.
All delays measured on localhost with release build and no tls