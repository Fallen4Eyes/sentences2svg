# Sentences 2 svg
## Does what it says on the tin.
This takes in a file with some sentences and outputs numbered svgs.
There are 3 arguments to keep in mind.

``-f or --font`` This is a path to the font file you want to use. There are no default paths here as fonts have no uniform names.

``-i or --input`` This takes an input from either a file or through stdin using ``--``. The default for this is ``./lines.txt``

``-o or --output`` This specifies the output directory of the file as well as the format. By default the output is ``./output``
If the output directory does not exist when running, then it'll make the output directory and all sub-directories.
By default the svgs come out as ``0.svg, 1.svg, etc..`` if you want to change that, you can do this ``./output/put_any_{}_text_here.svg``
the ``{}`` will be replaced with the current index. as of now that's all it does. It's not very fancy.