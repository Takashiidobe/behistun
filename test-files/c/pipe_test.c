#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
  int pipefd[2];

  if (pipe(pipefd) != 0) {
    perror("pipe");
    return 1;
  }

  printf("pipe created\n");

  // Write to pipe
  const char *msg = "hello pipe";
  write(pipefd[1], msg, strlen(msg));

  // Read from pipe
  char buf[64];
  int n = read(pipefd[0], buf, sizeof(buf));

  if (n > 0 && strncmp(buf, msg, strlen(msg)) == 0) {
    printf("pipe communication works\n");
  }

  close(pipefd[0]);
  close(pipefd[1]);

  return 0;
}
