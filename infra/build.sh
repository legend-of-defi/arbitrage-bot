#!/bin/bash

# Install required Ansible collections
ansible-galaxy collection install community.general
ansible-galaxy collection install ansible.posix
ansible-galaxy collection install community.docker

# Run Ansible playbook
ansible-playbook -i inventory.yml \
    01-system.yml \
    04-stas.yml \
    06-fly.yml \
    07-geth.yml \
    08-lighthouse.yml \
    09-base.yml \
    10-reth.yml
