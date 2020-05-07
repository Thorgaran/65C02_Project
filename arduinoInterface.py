import serial
import struct
import time
import os.path

def readSerial():
    val = str(ser.readline().decode().strip('\r\n'))#Capture serial output as a decoded string
    print(val)
    try:
        intVal = int(val)
    except:
        intVal = 0
    return intVal

#ser = serial.Serial('COM6', 9600)

romFile = open('Assembly\\rom.bin', 'rb')
rom = bytearray(romFile.read())

consecutiveEA = 0
codeLength = 0
while (consecutiveEA != 3) and (codeLength <= (0x7fef + 3)):
    #print(rom[i])    
    if rom[codeLength] == 0xea:
        consecutiveEA += 1
    elif consecutiveEA > 0:
        consecutiveEA = 0
    codeLength += 1

codeLength -= 3
codeLengthInt = struct.pack('>h', codeLength)
time.sleep(1)
readSerial() #*******START*******

#print(codeLengthInt)
ser.write(codeLengthInt)
#ser.write(0xff)
readSerial() #newCodeLength
oldCodeLength = readSerial() #oldCodeLength

print("")

for i in range(codeLength):
    print("rom[" + str(i) + "]: " + str(rom[i]))
    romInt = struct.pack('>B', rom[i])
    ser.write(romInt)
print("...")
for i in range(0x7ffa, 0x8000):
    print("rom[" + str(i) + "]: " + str(rom[i]))
    romInt = struct.pack('>B', rom[i])
    ser.write(romInt)

print("")
readSerial()
print("")

minPrintInt = struct.pack('>h', 0)
maxPrintInt = struct.pack('>h', max(oldCodeLength, codeLength))
ser.write(minPrintInt)
ser.write(maxPrintInt)

readSerial() #*******PRINT EEPROM CONTENT*******

for i in range(((max(oldCodeLength, codeLength) - 1) // 16) + 1):
    readSerial()
print("...")
readSerial() #8ff0: