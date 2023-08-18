#!/bin/bash


function main {
	if [[ $EUID -ne 0 ]]; then
		echo "Скрипт должен быть запущен под sudo"
		exit 1
	fi
	
	packages=("curl", "xorg", "xinit")

	for package in "${packages[@]}"; do
		if sudo dpkg -s "$package" >/dev/null 2>&1; then
			sudo apt install "$package"
		fi
	done

	sudo mkdir /etc/xffplay
	sudo touch /etc/xffplay/token.txt
	sudo echo "1208" > /etc/xffplay/token.txt
	
	curl https://github.com/PuffyWithEyes/xffplay/releases/download/v0.0.0/xffplay
	chmod +x xffplay

	sudo mv xffplay /opt/

	echo -e "\e[4m\e[1mqtmpv\e[0m установлен в \e[0m\e[32m/opt\e[0m\e[1m. Для запуска пропишите \e[32m/opt/qtmpv\e[0m"
}


main

