#include "sys/_default_fcntl.h"
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/unistd.h>

int main(int argc, char* argv[])
{
    FILE* file = fopen("test.txt", "w+");
    if (file == NULL)
    {
        perror("Failed to open file");
        return -1;
    }

    const char* message = "Hello, World!\n";
    size_t message_length = 14; // Length of the message including newline
    size_t written = fwrite(message, 1, message_length, file);
    if (written != message_length)
    {
        perror("Failed to write to file");
        fclose(file);
        return -1;
    }

    if (fseek(file, 0, SEEK_SET) != 0)
    {
        perror("ERRoR IN SEEKING\n");
        return -1;
    }


    char buffer[50];
    size_t read_bytes = fread(buffer, 1, message_length, file);
    if (read_bytes != message_length)
    {
        printf("Failed to read from file: %zd\n", read_bytes);
        fclose(file);
        return -1;
    }

    buffer[read_bytes] = '\0'; // Null-terminate the string
    printf("Read from file: %s", buffer);

    fclose(file);

}
