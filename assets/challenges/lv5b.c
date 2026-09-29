// ch-go: имитация Go-бинаря: имена функций в таблице (pclntab-style)
#include <stdio.h>
#include <string.h>

static const char *fnames[] = {"runtime.main","main.init","main.checkPass","main.util.xorkey"};
static const unsigned char keytab[] = {0x3D, 0x36, 0x28, 0x20, 0x38, 0x74};

static unsigned char g_state = 0;

static unsigned char xorkey(int i) { return keytab[i % 6] ^ 0x20; }

int checkPass(const char *s) {
  if (strlen(s) != 6) return 0;
  for (int i = 0; i < 6; i++) {
    unsigned char c = s[i];
    c = c ^ xorkey(i);
    c = c + i;
    if (c != 0x7A) return 0;
  }
  g_state = 1;
  return g_state;
}

int main(void) {
  char buf[16];
  printf("password: ");
  scanf("%15s", buf);
  if (checkPass(buf)) puts("Correct! FLAG{go_pclnt4b}");
  else puts("Wrong");
  return 0;
}
