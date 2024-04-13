# Artist
This is a program that takes control of your mouse and keyboard to paint an image in microsoft paint. It uses enigo for mouse and keyboard control and image for reading from the image.

## How to use
You are supposed to call this program from the command line and provide it with a path to an image, you can optionally provide a tolerance value (the higher the worse the quality, defaults to 5.0). I would not recommend setting this to a lower value than 5 as then the program takes way too long. A call could look like this:
`artist.exe "C:\path\to\image.png" 2.0`

After you have run the program you have to go into paint and move your mouse to one corner of where the painting is going to be and then press left control. After this you have to move to the other corner and again press left control. Now all you need to do is move the mouse over the black color preset (in the grid of colors) and press left control for the final time to get the program to start painting.

To stop the program simply move your mouse while it is painting or wait for it to finish.

## Todo

* Make the program create colors it needs when first filling custom color buffer.