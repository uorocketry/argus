# MS5611-01BA03 Driver Specifications

This document outlines the necessary specifications extracted from the datasheet for developing a software driver for the MS5611-01BA03 barometric pressure sensor.

## 1. General Information

*   **Device:** MS5611-01BA03 Barometric Pressure Sensor
*   **Key Features:** High resolution (10 cm), I2C/SPI interface, integrated 24-bit ADC, internal oscillator, factory calibration.
*   **Sensing:** Pressure and Temperature

## 2. Electrical Characteristics (Relevant to Driver)

*   **Supply Voltage (VDD):** 1.8 V to 3.6 V (Typical 3.0 V used for some specs)
*   **Supply Current (IDD @ 3V, 1 sample/sec):**
    *   Depends on Oversampling Ratio (OSR)
    *   OSR 4096: 12.5 µA (typ)
    *   OSR 2048: 6.3 µA (typ)
    *   OSR 1024: 3.2 µA (typ)
    *   OSR 512: 1.7 µA (typ)
    *   OSR 256: 0.9 µA (typ)
*   **Peak Supply Current:** 1.4 mA (during conversion)
*   **Standby Current (@ 25°C):** 0.02 µA (typ), < 0.15 µA (max)
*   **Internal Oscillator:** Yes, no external clock required for sensor operation.

## 3. Communication Interfaces

The sensor supports both SPI and I2C interfaces, selected by the `PS` pin.

*   **Protocol Selection (Pin 2 - PS):**
    *   `PS` High (VDD): I²C Mode
    *   `PS` Low (GND): SPI Mode

### 3.1. SPI Interface

*   **Pins:**
    *   `SCLK` (Pin 8): Serial Clock Input
    *   `SDI` (Pin 7): Serial Data Input
    *   `SDO` (Pin 6): Serial Data Output
    *   `CSB` (Pin 4): Chip Select (Active Low)
*   **Modes:** Supports SPI Mode 0 (CPOL=0, CPHA=0) and Mode 3 (CPOL=1, CPHA=1).
*   **Max Clock Speed (SCLK):** 20 MHz
*   **Data Transfer:** MSB first for both commands and data.
*   **Chip Select (CSB):** Must be low to enable communication. Can be kept low during a command sequence or de-asserted (high) between commands or after conversion completion. For best noise performance, keep the SPI bus idle during ADC conversions.

### 3.2. I²C Interface

*   **Pins:**
    *   `SCLK` (Pin 8): Serial Clock Input
    *   `SDA` (Pin 7): Serial Data Input/Output (Bidirectional)
    *   `CSB` (Pin 4): Determines LSB of I²C address. Must be connected to VDD or GND (cannot be left floating).
*   **Max Clock Speed (SCLK):** Standard I2C speeds likely supported (datasheet mentions up to 20 MHz interface, but typically I2C is 100kHz, 400kHz, sometimes 1MHz). Test specific speed.
*   **Device Address:** 7-bit address format: `111011C`
    *   If `CSB` is Low (GND): Address = `1110110` (0x76)
    *   If `CSB` is High (VDD): Address = `1110111` (0x77)
    *   The R/W bit is appended for read (1) or write (0) operations.
        *   CSB=0: Write Address = `0xEC`, Read Address = `0xED`
        *   CSB=1: Write Address = `0xEE`, Read Address = `0xEF`
*   **Data Transfer:** MSB first. Requires ACK from sensor after address/command bytes, and ACK from master after reading data bytes (except the last one, which requires NACK).

## 4. Commands

The sensor has 5 basic command types. Commands are 1 byte (8 bits).

### 4.1. Command List (SPI & I2C)

| Command Description        | OSR   | Hex Value | Notes                                      |
| :------------------------- | :---- | :-------- | :----------------------------------------- |
| Reset                      | N/A   | `0x1E`    | Reloads PROM calibration data.             |
| Convert D1 (Pressure)      | 256   | `0x40`    | Starts pressure conversion.                |
| Convert D1 (Pressure)      | 512   | `0x42`    | Starts pressure conversion.                |
| Convert D1 (Pressure)      | 1024  | `0x44`    | Starts pressure conversion.                |
| Convert D1 (Pressure)      | 2048  | `0x46`    | Starts pressure conversion.                |
| Convert D1 (Pressure)      | 4096  | `0x48`    | Starts pressure conversion.                |
| Convert D2 (Temperature)   | 256   | `0x50`    | Starts temperature conversion.             |
| Convert D2 (Temperature)   | 512   | `0x52`    | Starts temperature conversion.             |
| Convert D2 (Temperature)   | 1024  | `0x54`    | Starts temperature conversion.             |
| Convert D2 (Temperature)   | 2048  | `0x56`    | Starts temperature conversion.             |
| Convert D2 (Temperature)   | 4096  | `0x58`    | Starts temperature conversion.             |
| ADC Read                   | N/A   | `0x00`    | Reads the 24-bit result of last conversion. |
| PROM Read (Address 0)      | N/A   | `0xA0`    | Reads 16-bit PROM Word 0 (Reserved).       |
| PROM Read (Address 1 / C1) | N/A   | `0xA2`    | Reads 16-bit PROM Word 1 (SENST1).         |
| PROM Read (Address 2 / C2) | N/A   | `0xA4`    | Reads 16-bit PROM Word 2 (OFFT1).          |
| PROM Read (Address 3 / C3) | N/A   | `0xA6`    | Reads 16-bit PROM Word 3 (TCS).            |
| PROM Read (Address 4 / C4) | N/A   | `0xA8`    | Reads 16-bit PROM Word 4 (TCO).            |
| PROM Read (Address 5 / C5) | N/A   | `0xAA`    | Reads 16-bit PROM Word 5 (TREF).           |
| PROM Read (Address 6 / C6) | N/A   | `0xAC`    | Reads 16-bit PROM Word 6 (TEMPSENS).       |
| PROM Read (Address 7 / CRC)| N/A   | `0xAE`    | Reads 16-bit PROM Word 7 (Serial/CRC).     |

### 4.2. Command Execution Notes

*   **Reset:** Send `0x1E`. Wait required time (see Timing section) for PROM reload. Must be done once after power-on.
*   **Conversion:** Send a `Convert D1` or `Convert D2` command. The sensor becomes busy. Wait for the conversion time (see Timing section).
*   **ADC Read:** After conversion time, send `0x00`. Read the 24-bit result (MSB first). If read before conversion is complete, or read twice, result will be `0`. Sending a new conversion command during an ongoing conversion yields incorrect results.
*   **PROM Read:** Send the appropriate `0xA*` command. Read the 16-bit result (MSB first). PROM should be read after Reset to get calibration coefficients.

## 5. Data Acquisition and Processing

### 5.1. Typical Sequence

1.  Power on the sensor.
2.  Send `Reset` command (`0x1E`).
3.  Wait for reset completion (2.8 ms).
4.  Read all 8 PROM words (`0xA0` to `0xAE`) to get calibration coefficients (C1-C6) and CRC. Store these coefficients.
5.  *Optional:* Validate CRC (see Section 6).
6.  Send `Convert D2` command (e.g., `0x58` for OSR=4096).
7.  Wait for temperature conversion time (e.g., 8.22 ms typ for OSR=4096).
8.  Send `ADC Read` command (`0x00`).
9.  Read 24-bit raw temperature value (D2).
10. Send `Convert D1` command (e.g., `0x48` for OSR=4096).
11. Wait for pressure conversion time (e.g., 8.22 ms typ for OSR=4096).
12. Send `ADC Read` command (`0x00`).
13. Read 24-bit raw pressure value (D1).
14. Calculate temperature and compensated pressure using D1, D2, and C1-C6 (see Section 5.2).
15. Repeat steps 6-14 for continuous measurements.

### 5.2. Pressure and Temperature Calculation

**(Based on Figure 2 and Figure 3, requires 64-bit integer support for intermediate values)**

**Variables:**
*   `C1` to `C6`: 16-bit unsigned integers from PROM.
*   `D1`: 24-bit unsigned raw pressure from ADC.
*   `D2`: 24-bit unsigned raw temperature from ADC.

**First Order Calculation:**
1.  `dT = D2 - C5 * 2^8` (Result is signed 32-bit)
2.  `TEMP = 2000 + dT * C6 / 2^23` (Result is signed 32-bit, represents temp in 0.01 °C, e.g., 2007 = 20.07°C)
3.  `OFF = C2 * 2^16 + (C4 * dT) / 2^7` (Intermediate result needs signed 64-bit)
4.  `SENS = C1 * 2^15 + (C3 * dT) / 2^8` (Intermediate result needs signed 64-bit)
5.  `P = (D1 * SENS / 2^21 - OFF) / 2^15` (Result is signed 32-bit, represents pressure in 0.01 mbar, e.g., 100009 = 1000.09 mbar)

**Second Order Temperature Compensation (Recommended for better accuracy):**
Apply *after* calculating `TEMP`, `OFF`, `SENS` from the first order steps.
1.  Initialize `T2 = 0`, `OFF2 = 0`, `SENS2 = 0`.
2.  **If `TEMP < 2000`:**
    *   `T2 = (dT * dT) / 2^31` (Use 64-bit intermediate for `dT*dT`)
    *   `OFF2 = 5 * (TEMP - 2000)^2 / 2^1` (Use 64-bit intermediate)
    *   `SENS2 = 5 * (TEMP - 2000)^2 / 2^2` (Use 64-bit intermediate)
    *   **If `TEMP < -1500`:**
        *   `OFF2 = OFF2 + 7 * (TEMP + 1500)^2` (Use 64-bit intermediate)
        *   `SENS2 = SENS2 + 11 * (TEMP + 1500)^2 / 2^1` (Use 64-bit intermediate)
3.  **Apply Corrections:**
    *   `TEMP = TEMP - T2`
    *   `OFF = OFF - OFF2`
    *   `SENS = SENS - SENS2`
4.  **Recalculate Final Pressure:**
    *   `P = (D1 * SENS / 2^21 - OFF) / 2^15` (Result is signed 32-bit, pressure in 0.01 mbar)

**Final Output:**
*   Temperature = `TEMP / 100.0` (°C)
*   Pressure = `P / 100.0` (mbar)

## 6. PROM and Calibration Data

*   **Structure:** 128 bits (8 words of 16 bits).
    *   Word 0: Reserved / Factory Data
    *   Word 1-6: Calibration Coefficients C1-C6 (16-bit unsigned)
    *   Word 7: Upper bits Serial Code, Lower 4 bits CRC.
*   **Coefficients:**
    *   C1: Pressure sensitivity (SENST1)
    *   C2: Pressure offset (OFFT1)
    *   C3: Temperature coefficient of pressure sensitivity (TCS)
    *   C4: Temperature coefficient of pressure offset (TCO)
    *   C5: Reference temperature (TREF)
    *   C6: Temperature coefficient of the temperature (TEMPSENS)
*   **CRC:** 4-bit CRC calculated over PROM words 0-7 (excluding the CRC bits themselves). Checksum algorithm details are in Application Note AN520. Can be used to verify PROM data integrity after reading.

## 7. Timing Specifications

*   **Reset Command (`0x1E`) Reload Time:** 2.8 ms (max) - must wait this long after sending reset before next command.
*   **Conversion Time (tc):** Depends on OSR. Time between sending `Convert` command and data being ready for `ADC Read`.

| OSR   | Min (ms) | Typ (ms) | Max (ms) |
| :---- | :------- | :------- | :------- |
| 4096  | 7.40     | 8.22     | 9.04     |
| 2048  | 3.72     | 4.13     | 4.54     |
| 1024  | 1.88     | 2.08     | 2.28     |
| 512   | 0.95     | 1.06     | 1.17     |
| 256   | 0.48     | 0.54     | 0.60     |

*   **ADC Read:** Data (24 bits for ADC, 16 bits for PROM) is clocked out synchronously with SCLK after the `ADC Read` or `PROM Read` command is sent.

## 8. Resolution

*   **Pressure ADC:** 24 bits
*   **Temperature ADC:** 24 bits
*   **Pressure Resolution (RMS @ 3V, 25°C):** Depends on OSR.
    *   OSR 4096: 0.012 mbar
    *   OSR 2048: 0.018 mbar
    *   OSR 1024: 0.027 mbar
    *   OSR 512: 0.042 mbar
    *   OSR 256: 0.065 mbar
*   **Temperature Resolution (RMS @ 3V, 25°C):** Depends on OSR.
    *   OSR 4096: 0.002 °C
    *   OSR 2048: 0.003 °C
    *   OSR 1024: 0.005 °C
    *   OSR 512: 0.008 °C
    *   OSR 256: 0.012 °C
*   **Altitude Resolution:** High resolution mode (OSR 4096) typically provides ~10 cm resolution.

## 9. Pin Configuration Summary

| Pin | Name    | Type   | Function (SPI Mode)              | Function (I2C Mode)             |
| :-- | :------ | :----- | :------------------------------- | :------------------------------ |
| 1   | VDD     | P      | Positive Supply Voltage          | Positive Supply Voltage         |
| 2   | PS      | I      | Protocol Select (Tie Low)        | Protocol Select (Tie High)      |
| 3   | GND     | G      | Ground                           | Ground                          |
| 4   | CSB     | I      | Chip Select (Active Low)         | Address Select LSB (Tie High/Low)|
| 5   | NC      | -      | No Connect (Internal Connection) | No Connect (Internal Connection)|
| 6   | SDO     | O      | Serial Data Output               | No Connect (NC)                 |
| 7   | SDI/SDA | I/O    | Serial Data Input                | Serial Data I/O                 |
| 8   | SCLK    | I      | Serial Clock                     | Serial Clock                    |

## 10. Important Hardware Notes

*   **Decoupling Capacitor:** A 100 nF ceramic capacitor must be placed close to the VDD pin (Pin 1) and connected to GND (Pin 3) to stabilize the power supply during conversions and ensure accuracy.
*   **I2C Pull-up Resistors:** Required on SDA and SCLK lines (typical values 4.7kΩ - 10kΩ).
*   **PS Pin:** Must be tied high or low to select the interface; do not leave floating.
*   **CSB Pin (I2C Mode):** Must be tied high or low to set the address; do not leave floating.