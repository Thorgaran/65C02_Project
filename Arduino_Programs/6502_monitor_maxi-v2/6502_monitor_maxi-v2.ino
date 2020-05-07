const char ADDR[]= {22,24,26,28,30,32,34,36,38,40,42,44,52,50,48,46};
const char DATA[]= {23,27,25,31,29,33,35,37};
#define CLOCK 2
#define READ_WRITE 39

void setup() {
  
  for(int n=0; n < 16; n += 1){
    pinMode(ADDR[n], INPUT);
  }
  for(int n = 0; n < 8; n += 1){
    pinMode(DATA[n],INPUT);
  }
  pinMode(CLOCK,INPUT);
  pinMode(READ_WRITE, INPUT);
  
  attachInterrupt(digitalPinToInterrupt(CLOCK), onClock, RISING);
  
  Serial.begin(9600);
}

void onClock() {
  char output[15];
  Serial.print("-->");
  unsigned int address = 0;
  for (int n = 15; n >= 0; n -= 1) {
    int bit = digitalRead(ADDR[n]);
    Serial.print(bit);
    address = (address << 1) + bit;  
  }
  
  Serial.print("   ");
  
  unsigned int data = 0;
  for (int n = 7; n >= 0; n -= 1) {
    int bit = digitalRead(DATA[n]) ? 1: 0;
    Serial.print(bit);
    data = (data << 1) + bit;
  }
  
  sprintf(output, "   %04x   %c   %02x",address, digitalRead(READ_WRITE) ? 'r' : 'W', data);
  Serial.println(output);
  
  
    
}

void loop(){

}
