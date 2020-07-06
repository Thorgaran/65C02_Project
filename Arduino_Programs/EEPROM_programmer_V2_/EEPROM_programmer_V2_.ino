//definition des branchemment des min de l'EEPROM sur l'arduino
#define WRITE_EN 6
#define OUTPUT_EN 5
#define CHIP_EN 4

#define NB_DATA 8
#define NB_ADDR 15
#include <digitalWriteFast.h>

const int ADDR[15] = {22,24,26,28,30,32,34,36,46,44,50,42,38,48,40};
const int DATA[8] = {27,25,23,29,31,33,35,37};

//*****FONCTIONS*****

void setPinMode(int pin[], int pinSize, int state){
  //définit au niveau de l'arduino si les pin sont des entrées ou des sorties
  //pin[]: ADDR[] ou DATA[], pinSize : taille du tableau 15 ou 8, state : 1 pour OUTPUT et 0 pour INPUT
  if (state){
    for (int i = 0; i < pinSize; i++){
      pinMode(pin[i], OUTPUT);
    }
  }
  else{
    for (int i = 0; i < pinSize; i++){
      pinMode(pin[i], INPUT);
    }
  }
  delayMicroseconds(1);
  //delais à définir
}

void setData(byte dataValue){
  //met la valeur demandée sur les pin de l'arduino qui sont connecté au data de l'EEPROM
  //dataValue : valeur a envoyer sur les data de l'EEPROM en binaire
  //******WARNING avant d'utiliser cette fontion il faut impérativement que les pins data soit definit comme OUTPUT*****
  //placer cette ligne avant : setPinMode(DATA, 8, OUTPUT);

  for (int i = 0; i < NB_DATA; i++){
    digitalWrite(DATA[i], (dataValue & 1));
    dataValue = (dataValue >> 1);
  }
}

void setAddress(int addrValue){
  //met la valeur demandée sur les pin de l'arduino qui sont connecté aux addresse de l'EEPROM
  //addrValue : valeur a envoyer sur les addresse de l'EEPROM en décimal

  for (int i = 0; i < NB_ADDR; i++){
    digitalWrite(ADDR[i], (addrValue & 1));
    addrValue = (addrValue >> 1);
  }
  /*for (int i = 0; i < NB_ADDR; i++){
    if (addrValue != 0){
      digitalWrite(ADDR[i], (addrValue%2));
      addrValue = addrValue/2;
    }
    else{
      digitalWrite(ADDR[i], 0);
    }
  }*/
}

byte readData(){
  //lit la valeur renvoyée par l'EEPROM sur les pin data de l'arduino
  //retourne la  valeur lue en BINAIRE !!
  //******WARNING avant d'utiliser cette fontion il faut impérativement que les pins data soit definit comme INPUT*****
  //placer cette ligne avant : setPinMode(DATA, 8, INPUT);
  byte dataValue = 15;
  for (int i = NB_DATA - 1; i >= 0; i--){
    //on lit dans l'autres sens pour que la valeur de dataValue soit dans le sens de lecture conventionnel (bit de poid le plus fort a gauche)
    dataValue = (dataValue << 1 ) | digitalRead(DATA[i]);
  }
  return dataValue;
}

int changeBase (int n, int baseInitial, int baseFinal)
{
  //prend un entier nInitial en base baseInital et renvoi de nombre en base baseFinal
  //n : nombre entier sur lequel on veux realiser un changment de base
  //baseInitail : base dans laquel le nombre nInitial est ecrit <=10 et >=2
  //baseFinal : base dans laquel on veut ecrire le nombre retourner par la fonction
  //renvoit le nombre n dans la base naseFinal
  //*****le marche pas pour la base 16 a améliorer****
  
    int nFinal = 0, i=0, nBase10 = 0;
    while (n != 0){
        nBase10 = nBase10 + ( n % 10 )*pow(baseInitial, i);
        i ++;
        n = (n- (n%10))/10;
    }
    i = 0;
    while(nBase10 != 0){
        nFinal = nFinal + (nBase10%baseFinal)*pow(10, i);
        i++;
        nBase10 = nBase10/baseFinal;
    }
    return nFinal;
}

byte readOneEEPROMAddr(int address){
  //lit la valeur d'une addresse sur l'EEPROM
  //address : l'adresse que l'on veut lire, en binaire !!
  //return : la valeur en binaire lue sur les data de l'EEPROM
  //tout les delais propre a l'EEPROM sont géré dans cette fonction
  
  byte data = 0;
  setAddress(address);
  digitalWrite(CHIP_EN, LOW);
  digitalWrite(OUTPUT_EN, LOW);
  delayMicroseconds(1);//a redéfinir minimun 350ns (sans compter les temps d'exécution) / 70ns plutot!
  data = readData();
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(OUTPUT_EN, HIGH);//on remet dans l'état initial

  return data;
  
}

void writeOneEEPROMAddress(int address, byte data){
  //Ecrit sur l'EEPROM a l'adresse indiqué la valeur data
  //address : adresse a laquel on veut écrire en décimal
  //data : valeur a écrire une l'EEPROM en binaire

  setAddress(address);
  digitalWrite(OUTPUT_EN, HIGH);
  digitalWrite(CHIP_EN, LOW);
  setData(data);
  digitalWriteFast(6, LOW);
    //delais a definir /normalement c'est good
  digitalWriteFast(6, HIGH);
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(OUTPUT_EN, LOW);
  delay(10);//10 ms de delais entre deux écriture
  
}

void writeEaEverywhere(int mini, int maxi){
  //Ecrit ea dans l'eeprom entres les adresses 0x0 et max
  
  for (int j = mini; j < maxi; j++){
    writeOneEEPROMAddress(j, 0xea);
  }
}

void writeConsecutiveEEPROMAddress(int firstAddress,byte dataValue[], int sizeData){
  //ecrit sur l'EEPROM a des adresse qui se suivent les valeur contenue dans le tableau dataValue
  //firstAddress : la première adresse ou on commence a écrire en décimal
  //dataValue : tableau contenant les valeur a écrire sur l'EEPROM les une a la suite des autres
  //sizeData : nombre de valeur a écrire
  
  Serial.print("-->Start to write ");
  Serial.print(sizeData);
  Serial.println(" value on EEPROM");

  for (int i = 0; i < sizeData; i++){
    writeOneEEPROMAddress((firstAddress + i), dataValue[i]);
  }
  Serial.println("Writing finish");
}

void printEEPROMContent(unsigned int firstAddr, unsigned int lastAddr){
  //affiche dans le moniteur série en hexadécimal les valeur de l'EEPROM sur toute les address entre firstAddr et lasrAddr
  //firstAddr : première addresse à lire
  //lastAddr : dèniere addresse à lire

  //afin d'afficher 16 informations par ligne, on modifie firstAddr et lastAddr pour qu'ils soient multipple de 16
  firstAddr = firstAddr - (firstAddr%16);//multiple de 16 inférieur
  lastAddr = lastAddr - 1 + (16 - (lastAddr%16));//multiple de 16 supérieur -1
  int dataValue[16];
  //Serial.println(" ");
  for (unsigned int base = firstAddr; base < lastAddr; base +=16){
    for (unsigned int offset = 0; offset < 16; offset ++){
      dataValue[offset] = readOneEEPROMAddr(base + offset);
    }
    char lineToPrint[200];
    sprintf(lineToPrint, "%04x: %02x %02x %02x %02x %02x %02x %02x %02x    %02x %02x %02x %02x %02x %02x %02x %02x", base, dataValue[0], dataValue[1], dataValue[2], 
            dataValue[3], dataValue[4], dataValue[5], dataValue[6], dataValue[7], dataValue[8], dataValue[9], dataValue[10], dataValue[11], dataValue[12], dataValue[13], 
            dataValue[14], dataValue[15]);
    Serial.println(lineToPrint);
  }
}

int countProgramSize () {
  int nbConsecutiveEA = 0;
  int addr = 0;
  byte data; 
  while (nbConsecutiveEA < 3 && addr <= 0x7fef){ //Change to 00ff to test without EEPROM
  //while (nbConsecutiveEA < 3 && addr <= 0x00ff){
    data = readOneEEPROMAddr(addr);
    addr++;
    if (data == 0xea){
      nbConsecutiveEA ++; 
    }
    else if (nbConsecutiveEA > 0){
      nbConsecutiveEA = 0;
    }
  }
  addr -= 3;
  return addr; 
}

void writeFromSerial (unsigned int mini, unsigned int maxi){
  byte data;
  for (long addr = mini; addr < maxi; addr++){
    while (Serial.available() == 0) {
      delay(1);
    }
    Serial.readBytes(&data, 1);
    Serial.println(data);
    writeOneEEPROMAddress(addr, data);
  }
}

unsigned int readAddrFromSerial () {
  unsigned int addr;
  byte buffer;
  while (Serial.available() == 0) {
    delay(1);
  }
  Serial.readBytes(&buffer, 1);
  addr = (buffer << 8);
  Serial.readBytes(&buffer, 1);
  addr += buffer;
  return addr;
}

//*******FIN FONCTIONS******

//*******DEBUT CODE******
void setup() {
  int oldCodeLength, newCodeLength, minPrint, maxPrint;
  
  //on met les trois pin de controle dans un état "initial" le plus tot possible  -->OE 1, CE 1, WE 1
  digitalWrite(OUTPUT_EN, HIGH);
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(WRITE_EN, HIGH);
  pinMode(OUTPUT_EN, OUTPUT);
  pinMode(WRITE_EN, OUTPUT);
  pinMode(CHIP_EN, OUTPUT);
  
  setPinMode(ADDR, NB_ADDR, OUTPUT);
  setPinMode(DATA, NB_DATA, INPUT);//a faire avant de vouloire lire l'EEPROM

  delay(1000);
  
  Serial.begin(9600);//initialisation du moniteur série
  Serial.println("*******START*******");
  
  newCodeLength = readAddrFromSerial();
  Serial.println(newCodeLength);
  
  //******Sequence de lecture*****
  setPinMode(DATA, NB_DATA, INPUT);//a faire avant de vouloire lire l'EEPROM
  digitalWrite(OUTPUT_EN, HIGH);
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(WRITE_EN, HIGH);//etat initial pour la lecture
  oldCodeLength = countProgramSize();
  Serial.println(oldCodeLength);
  
  //******Séquence d'écriture******
  setPinMode(DATA, NB_DATA, OUTPUT);
  digitalWrite(OUTPUT_EN, LOW);
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(WRITE_EN, HIGH);//état initial pour l'écriture

  //byte programme[10] = {0xca, 0xfa, 0x10, 0xde, 0x7a, 0xc0, 0xf1, 0xf1, 0xf1, 0x31};
  //writeConsecutiveEEPROMAddress(8, programme, 10);
  //writeOneEEPROMAddress(0x7fff,0xea);
  writeFromSerial(0, newCodeLength);
  Serial.println("First half worked");
  writeFromSerial(0x7ffa, 0x8000);
  if (oldCodeLength > newCodeLength) {
    writeEaEverywhere(newCodeLength, oldCodeLength);
    Serial.println("Wrote some EAs!");
  }
  else {
    Serial.println("Longer code, nothing to write...");
  }
  
  //******Sequence de lecture*****
  setPinMode(DATA, NB_DATA, INPUT);//a faire avant de vouloire lire l'EEPROM
  digitalWrite(OUTPUT_EN, HIGH);
  digitalWrite(CHIP_EN, HIGH);
  digitalWrite(WRITE_EN, HIGH);//etat initial pour la lecture
  
  minPrint = readAddrFromSerial();
  maxPrint = readAddrFromSerial();

  Serial.println("*******PRINT EEPROM CONTENT*******");
  printEEPROMContent(minPrint, maxPrint); // on affiche le contenu de l'EEPROM entre les addresses 0 et maxPrint (longueur du code)
  printEEPROMContent(0x7ff0, 0x7fff);     // on affiche le contenu de l'EEPROM sur ses 16 dernières addresses
}

void loop(){
  
}
//********FIN CODE********
