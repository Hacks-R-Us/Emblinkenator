/**
 * Based on https://github.com/thehookup/Holiday_LEDS
 */

/*****************  NEEDED TO MAKE NODEMCU WORK ***************************/
#define FASTLED_INTERRUPT_RETRY_COUNT 0
#define FASTLED_ESP8266_NODEMCU_PIN_ORDER

/******************  LIBRARY SECTION *************************************/
#include <FastLED.h>
#include <ESP8266WiFi.h>
#include <ESP8266mDNS.h>
#include <WiFiUdp.h>
#include "Settings.h"

/*****************  LED LAYOUT AND SETUP *********************************/

#define DATA_PIN 8
#define CLK_PIN 5

#define NUM_LEDS 50

/***********************  WIFI AND MQTT SETUP *****************************/
const char *ssid = WIFI_SSID;
const char *password = WIFI_PASSWORD;
const char *hostName = DEVICE_HOSTNAME;

/*****************  DECLARATIONS  ****************************************/
WiFiClient espClient;
WiFiUDP udp;
unsigned int localPort = 3663;

CRGB leds[NUM_LEDS];
char packetBuffer[255];
char ReplyBuffer[] = "ack";

/*****************  GLOBAL VARIABLES  ************************************/

const int ledPin = 5;   //marked as D1 on the board

byte brightness = 196;

void setup_wifi() {
	// We start by connecting to a WiFi network
	Serial.println();
	Serial.print("Connecting to ");
	Serial.println(ssid);

	WiFi.hostname(hostName);
	// Configures static IP address
	if (!WiFi.config(local_IP, gateway, subnet)) {
		Serial.println("STA Failed to configure");
	}

	WiFi.begin(ssid, password);

	while (WiFi.status() != WL_CONNECTED) {
		delay(500);
		Serial.print(".");
	}

	Serial.println("");
	Serial.println("WiFi connected");
	Serial.println("IP address: ");
	Serial.println(WiFi.localIP());

	udp.begin(localPort);
}

void setup()
{
	Serial.begin(115200);
	pinMode(2, OUTPUT);
	digitalWrite(2, HIGH);

	FastLED.addLeds<WS2801, DATA_PIN, CLK_PIN, RGB>(leds, NUM_LEDS);

	WiFi.setSleepMode(WIFI_NONE_SLEEP);
	WiFi.mode(WIFI_STA);
	setup_wifi();

	for (int i = 0; i < 3; i++) {
		digitalWrite(2, LOW);
		delay(1000);
		digitalWrite(2, HIGH);
		delay(1000);
	}
}

void loop()
{
	int packetSize = udp.parsePacket();
	if (packetSize) {
		int length = udp.read(packetBuffer, 255);
		if (length == NUM_LEDS * 3) {
			for (int i = 0; i < length / 3; i++) {
				leds[i] = CRGB(packetBuffer[i * 3], packetBuffer[i * 3 + 1], packetBuffer[i * 3 + 2]);
			}
		}
		udp.beginPacket(udp.remoteIP(), udp.remotePort());
		udp.write(ReplyBuffer);
		udp.endPacket();
	}
	FastLED[0].showLeds(brightness);
}
