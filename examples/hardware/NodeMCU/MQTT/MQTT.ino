/**
 * Based on https://github.com/thehookup/Holiday_LEDS
 */

/*****************  NEEDED TO MAKE NODEMCU WORK ***************************/
#define FASTLED_INTERRUPT_RETRY_COUNT 0
#define FASTLED_ESP8266_NODEMCU_PIN_ORDER

/******************  LIBRARY SECTION *************************************/
#include <FastLED.h>
#include <SimpleTimer.h>
#include <PubSubClient.h>
#include <ESP8266WiFi.h>
#include <ESP8266mDNS.h>
#include <WiFiUdp.h>
#include "Settings.h"

#define concat(first, second) first second
#define topic(name) MQTT_TOPIC_ROOT "/" name

#define TOPIC_BRIGHTNESS topic("brightness")
#define TOPIC_EFFECT topic("effect")
#define TOPIC_POWER topic("power")
#define TOPIC_HEARTBEAT "heartbeat/" MQTT_TOPIC_ROOT

#define EFFECT_NONE "none"
#define EFFECT_COLOUR_CHASE "colour-chase"
#define EFFECT_LED_LOCATOR "locator"

#define EFFECT_SEPARATOR "|"

/*****************  LED LAYOUT AND SETUP *********************************/

#define DATA_PIN 8
#define CLK_PIN 5

#define NUM_LEDS 50

/***********************  WIFI AND MQTT SETUP *****************************/
const char *ssid = WIFI_SSID;
const char *password = WIFI_PASSWORD;
const char *hostName = DEVICE_HOSTNAME;
const char *mqtt_server = MQTT_SERVER;
const int mqtt_port = MQTT_PORT;
const char *mqtt_user = MQTT_USER;
const char *mqtt_pass = MQTT_PASSWORD;
const char *mqtt_client_name = MQTT_CLIENT_NAME;

/*****************  DECLARATIONS  ****************************************/
WiFiClient espClient;
PubSubClient client(espClient);
SimpleTimer timer;

CRGB leds[NUM_LEDS];

/*****************  GLOBAL VARIABLES  ************************************/

const int ledPin = 5;   //marked as D1 on the board

bool boot = true;

String currentEffect = EFFECT_COLOUR_CHASE;
bool showLights = true;

byte brightness = 196;
int patternUpdateInterval = 1000;

uint8_t fadeToBlackDuration = 0;
uint8_t fadeToBlackCurrentStep = 0;
uint8_t fadeToBlackSteps = 0;
int fadeToBlackTimeIntervalMs = 0;

int effectTimerId = -1;
uint8_t seqLightInSequenceCurrentPixel = 0;
int ledLocator = 0;

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
}

void reconnect()
{
	// Loop until we're reconnected
	int retries = 0;
	while (!client.connected()) {
		if (retries < 150) {
			Serial.print("Attempting MQTT connection...");
			// Attempt to connect
			if (client.connect(mqtt_client_name, mqtt_user, mqtt_pass)) {
				Serial.println("connected");
				// Once connected, publish an announcement...
				if (boot == true) {
					client.publish(TOPIC_HEARTBEAT, "Rebooted");
					boot = false;
				}

				if (boot == false) {
					client.publish(TOPIC_HEARTBEAT, "Reconnected");
				}

				client.subscribe(TOPIC_BRIGHTNESS);
				client.subscribe(TOPIC_EFFECT);
				client.subscribe(TOPIC_POWER);
			} else {
				Serial.print("failed, rc=");
				Serial.print(client.state());
				Serial.println(" try again in 5 seconds");
				retries++;

				// Wait 5 seconds before retrying
				delay(5000);
			}
		}

		if (retries > 1500) {
			ESP.restart();
		}
	}
}

String payloadToString(byte *payload, unsigned int length) {
	payload[length] = '\0';
	String newPayload = String((char *)payload);
	return newPayload;
}

int getMessageSeparatorPosition(String payload) {
	return payload.indexOf(EFFECT_SEPARATOR);
}

String getEffectName(String payload) {
	if (payload == EFFECT_COLOUR_CHASE) {
		return payload;
	}

	int separatorPosition = getMessageSeparatorPosition(payload);

	if (separatorPosition == -1) {
		return EFFECT_NONE;
	}

	String effect = payload.substring(0, separatorPosition);

	if (effect == EFFECT_LED_LOCATOR) {
		return effect;
	}

	return EFFECT_NONE;
}

void callback(char *topic, byte *payload, unsigned int length)
{
	String newTopic = topic;
	String newPayload = payloadToString(payload, length);

	if (newTopic == TOPIC_EFFECT) {
		if (length == NUM_LEDS * 3) {
			clearPattern();
			for (int i = 0; i < length / 3; i++) {
				leds[i] = CRGB(payload[i * 3], payload[i * 3 + 1], payload[i * 3 + 2]);
			}
		} else {
			int separatorPosition = getMessageSeparatorPosition(newPayload);
			String selectedEffect = getEffectName(newPayload);
			currentEffect = selectedEffect;
			
			if (currentEffect == EFFECT_LED_LOCATOR) {
				ledLocator = newPayload.substring(separatorPosition + 1, length).toInt();
			}

			setPattern();
		}
	}

	if (newTopic == TOPIC_BRIGHTNESS) {
		brightness = newPayload.toInt();
		FastLED[0].showLeds(brightness);
	}

	if (newTopic == TOPIC_POWER) {
		if (newPayload == "ON") {
			showLights = true;
			setPattern();
		}

		if (newPayload == "OFF") {
			showLights = false;
			currentEffect = EFFECT_NONE;
			setPattern();
		}
	}
}

void checkIn() {
	client.publish(TOPIC_HEARTBEAT, "ALIVE");
	timer.setTimeout(120000, checkIn);
}

void clearPattern () {
	if (effectTimerId != -1) {
		timer.deleteTimer(effectTimerId);
		effectTimerId = -1;
	}
}

void setPattern() {
	clearPattern();

	if (showLights == true)	{
		if (currentEffect == EFFECT_COLOUR_CHASE) {
			patternUpdateInterval = 100;
			seqLightInSequenceCurrentPixel = 0;
			cutToBlack();
			effectTimerId = timer.setInterval(patternUpdateInterval, lightInSequence);
		}

		if (currentEffect == EFFECT_LED_LOCATOR) {
			cutToBlack();
			showLocator();
		}

		if (currentEffect == EFFECT_NONE) {
			fadeToBlack(1, 2);
		}
	}

	if (showLights == false) {
		fadeToBlackBy(leds, NUM_LEDS, 255);
	}
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
	client.setServer(mqtt_server, mqtt_port);
	client.setCallback(callback);

	for (int i = 0; i < 3; i++) {
		digitalWrite(2, LOW);
		delay(1000);
		digitalWrite(2, HIGH);
		delay(1000);
	}

	timer.setTimeout(120000, checkIn);

	setPattern();
}

void loop()
{
	if (!client.connected()) {
		reconnect();
	}

	client.loop();
	timer.run();
	FastLED[0].showLeds(brightness);
}

void lightInSequence()
{
	if (seqLightInSequenceCurrentPixel > NUM_LEDS) {
		seqLightInSequenceCurrentPixel = 0;
		cutToBlack();
	}

	leds[seqLightInSequenceCurrentPixel] = CRGB(100, 100, 100);
	seqLightInSequenceCurrentPixel++;
}

void showLocator()
{
    if (ledLocator < NUM_LEDS) {
		cutToBlack();
		leds[ledLocator] = CRGB(100, 100, 100);
	}
}

void cutToBlack () {
	for (int i = 0; i < NUM_LEDS; i++) {
		leds[i] = CRGB(0, 0, 0);
	}
}

void fadeToBlack(uint8_t durationSec, uint8_t steps) {
	clearPattern();

	fadeToBlackSteps = steps;
	fadeToBlackCurrentStep = 1;
	fadeToBlackDuration = durationSec;
	fadeToBlackTimeIntervalMs = (uint8_t)(fadeToBlackDuration / fadeToBlackSteps) * 1000;
	fadeToBlackInterval();
}

void fadeToBlackInterval () {
	if (fadeToBlackCurrentStep == fadeToBlackSteps) {
		fadeToBlackCurrentStep = 0;
		fadeToBlackSteps = 0;
		fadeToBlackDuration = 0;
		fadeToBlackTimeIntervalMs = 0;
		setPattern();
	} else {
		fadeToBlackBy(leds, NUM_LEDS, (uint8_t)(255 / fadeToBlackSteps) * fadeToBlackCurrentStep);
		fadeToBlackCurrentStep++;
		timer.setTimeout(fadeToBlackTimeIntervalMs, fadeToBlackInterval);
	}
}
