#include <stdio.h>
#include <string.h>
int main(void){ char buf[32]; int i, ok = 1;
  printf("password: "); scanf("%31s", buf);
  if (strlen(buf) != 7) { puts("Wrong"); return 1; }
  const unsigned char enc[7] = {0xB5,0xB0,0xB7,0xB4,0xB1,0xBE,0xB4};
  for (i = 0; i < 7; i++) if ((unsigned char)buf[i] != (enc[i] ^ 0xFF)) ok = 0;
  if (ok) { puts("Correct! FLAG{lv2b_bytecmp}"); return 0; }
  puts("Wrong"); return 1; }