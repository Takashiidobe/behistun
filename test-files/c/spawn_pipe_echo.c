#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  int fds[2];
  assert(pipe(fds) == 0);
  pid_t pid = fork();
  assert(pid >= 0);
  if (pid == 0) {
    close(fds[0]);
    const char *msg = "child";
    write(fds[1], msg, 5);
    close(fds[1]);
    _exit(0);
  } else {
    close(fds[1]);
    char buf[8] = {0};
    read(fds[0], buf, sizeof(buf));
    printf("%s\n", buf);
    close(fds[0]);
    return 0;
  }
}
