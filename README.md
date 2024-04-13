# Artist
This is a program that takes control of your mouse and keyboard to paint an image in microsoft paint. It uses the enigo crate for mouse and keyboard control and the image crate for reading from the image. The program also uses the crates device_query and xcap to capture keyboard events and take screenshots respectively. If you want to share the painting the program will also save a screenshot to `out.png`.

## How to use
You are supposed to call this program from the command line and provide it with a path to an image, you can optionally provide a tolerance value (the higher the worse the quality, defaults to 5.0). I would not recommend setting this to a lower value than 5 as then the program takes way too long. A call could look like this:
`artist.exe "C:\path\to\image.png" 2.0`

After you have run the program you have to go into paint and move your mouse to one corner of where the painting is going to be and then press left control. After this you have to move to the other corner and again press left control. Now all you need to do is move the mouse over the black color preset (in the grid of colors) and press left control for the final time to get the program to start painting.

To stop the program simply move your mouse while it is painting or wait for it to finish.