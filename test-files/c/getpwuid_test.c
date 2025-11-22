#include <pwd.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  struct passwd *pw = getpwuid(getuid());
  if (!pw) {
    puts("no entry");
    return 1;
  }
  printf("%s %d %d %s\n", pw->pw_name, pw->pw_uid, pw->pw_gid,
         pw->pw_dir ? pw->pw_dir : "-");
  return 0;
}
